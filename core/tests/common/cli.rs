//! 差分テストのための、CLI の出力の形に整える層（ls・graph・read・check と、feature `write` の init・new・archive・mv・
//! check --fix）。
//!
//! CLI の段（R4）の前なので、Go の CLI と同じ引数を受けて core を呼び、Go と同じ標準出力・標準エラー・終了コードにする。
//! 出力の整形は core に置かない決まり（設計の 2 節）なので、ここに置き、R4 で CLI の crate に移す。
//! 表（ls・graph --all・check の既定の出力）は R4 で比べるので、ここでは作らない。

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use desk_core::check::Finding;
use desk_core::graph::{Direction, Walk, edges_via};
use desk_core::query::{Modified, Query, Where, days_before, split_comma};
use desk_core::quote::go_quote;
use desk_core::rules::{ALL_KINDS, describe};
use desk_core::{Doc, Edge, Kind, Tree, find_root};
use serde_json::Value;

/// 1 回の実行の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// Go の CLI と同じ引数で、core を呼んで同じ形の結果を作る。cwd は作業ディレクトリ。
pub fn run(cwd: &Path, args: &[&str]) -> Out {
    let mut o = Out {
        stdout: String::new(),
        stderr: String::new(),
        code: 0,
    };
    let res = match args[0] {
        "ls" => ls(&mut o, cwd, &args[1..]),
        "graph" => graph(&mut o, cwd, &args[1..]),
        "read" => read(&mut o, cwd, &args[1..]),
        "check" => check(&mut o, cwd, &args[1..]),
        #[cfg(feature = "write")]
        "init" => write::init(&mut o, cwd, &args[1..]),
        #[cfg(feature = "write")]
        "new" => write::new(&mut o, cwd, &args[1..]),
        #[cfg(feature = "write")]
        "archive" => write::archive(&mut o, cwd, &args[1..]),
        #[cfg(feature = "write")]
        "mv" => write::mv(&mut o, cwd, &args[1..]),
        c => panic!("このテストの層に無いコマンド: {c}"),
    };
    if let Err(e) = res {
        if !e.msg.is_empty() {
            o.stderr.push_str(&format!("desk: {}\n", e.msg));
        }
        o.code = e.code;
    }
    o
}

struct Fail {
    code: i32,
    msg: String,
}

fn fail(msg: impl ToString) -> Fail {
    Fail {
        code: 1,
        msg: msg.to_string(),
    }
}

fn usage(msg: impl ToString) -> Fail {
    Fail {
        code: 2,
        msg: msg.to_string(),
    }
}

/// フラグ（pflag と同じく、位置引数と混ぜてよい）。
struct Flags {
    pos: Vec<String>,
    vals: HashMap<String, Vec<String>>,
    on: HashSet<String>,
}

impl Flags {
    fn parse(args: &[&str], bools: &[&str]) -> Flags {
        let mut f = Flags {
            pos: Vec::new(),
            vals: HashMap::new(),
            on: HashSet::new(),
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            let Some(name) = a.strip_prefix("--") else {
                f.pos.push(a.to_string());
                continue;
            };
            if let Some((n, v)) = name.split_once('=') {
                f.vals.entry(n.into()).or_default().push(v.into());
            } else if bools.contains(&name) {
                f.on.insert(name.into());
            } else {
                let v = it.next().expect("フラグの値が無い");
                f.vals.entry(name.into()).or_default().push(v.to_string());
            }
        }
        f
    }

    fn str(&self, name: &str) -> Option<&str> {
        self.vals
            .get(name)
            .and_then(|v| v.last())
            .map(String::as_str)
    }

    fn int(&self, name: &str, default: i64) -> i64 {
        self.str(name).map_or(default, |s| s.parse().unwrap())
    }

    fn all(&self, name: &str) -> Vec<String> {
        self.vals.get(name).cloned().unwrap_or_default()
    }

    fn on(&self, name: &str) -> bool {
        self.on.contains(name)
    }
}

fn load(cwd: &Path) -> Result<Tree, Fail> {
    let root = find_root(cwd).map_err(fail)?;
    Tree::load(&root).map_err(fail)
}

/// 今日（Go の time.Now と同じく手元の時刻で）。
fn today() -> String {
    let out = Command::new("date").arg("+%F").output().unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn line(o: &mut Out, s: &str) {
    o.stdout.push_str(s);
    o.stdout.push('\n');
}

// ls

fn ls(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
    let f = Flags::parse(args, &["archived", "json", "paths"]);
    if f.pos.len() > 1 {
        return Err(usage("種類は 1 つ（カンマ区切りで複数）"));
    }
    let mut q = Query::default();
    if let Some(k) = f.pos.first() {
        for k in split_comma(k) {
            let Some(kind) = Kind::from_name(&k) else {
                let names: Vec<&str> = ALL_KINDS.iter().map(|k| k.as_str()).collect();
                return Err(usage(format!(
                    "知らない種類 {}（{}）",
                    go_quote(&k),
                    names.join(" ")
                )));
            };
            q.kinds.push(kind);
        }
    }
    for w in f.all("where") {
        q.wheres.push(Where::parse(&w).map_err(usage)?);
    }
    let level = f.int("level", -1);
    if level > 2 {
        return Err(usage("--level は 0・1・2"));
    }
    q.name = f.str("name").filter(|s| !s.is_empty()).map(str::to_string);
    q.project = f
        .str("project")
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    q.archived = f.on("archived");
    let (before, after) = (f.int("modified-before", -1), f.int("modified-after", -1));
    if before >= 0 || after >= 0 {
        let today = today();
        let day = |n: i64| (n >= 0).then(|| days_before(&today, n as u32).unwrap());
        q.modified = Some(Modified {
            before: day(before),
            after: day(after),
            today: today.clone(),
        });
    }
    let t = load(cwd)?;
    let hits = t.ls(&q);
    if f.on("paths") {
        for e in &hits {
            line(o, &e.info.path);
        }
    } else if level >= 0 {
        for (i, e) in hits.iter().enumerate() {
            if i > 0 && level > 0 {
                line(o, "");
            }
            if e.info.kind == Some(Kind::Project) {
                match t.get(&format!("{}/README.md", e.info.path)) {
                    Some(r) => read_doc(o, &t, r, &ReadOpt::level(level), true),
                    None => line(o, &format!("{}  [project]", e.info.path)),
                }
                continue;
            }
            read_doc(o, &t, e.doc.unwrap(), &ReadOpt::level(level), true);
        }
    } else if f.on("json") {
        let cols = split_comma(f.str("fields").unwrap_or(""));
        let rows: Vec<Value> = hits
            .iter()
            .map(|e| {
                let mut v = serde_json::to_value(e).unwrap();
                let m = v.as_object_mut().unwrap();
                for c in &cols {
                    if !m.contains_key(c) {
                        m.insert(c.clone(), Value::String(e.field_string(&t, c)));
                    }
                }
                v
            })
            .collect();
        o.stdout = serde_json::to_string_pretty(&rows).unwrap() + "\n";
    } else {
        panic!("ls の表は R4 で比べる");
    }
    Ok(())
}

// read

struct ReadOpt {
    level: i64,
    outline: bool,
    sections: Vec<String>,
}

impl ReadOpt {
    fn level(level: i64) -> ReadOpt {
        ReadOpt {
            level,
            outline: false,
            sections: Vec::new(),
        }
    }
}

fn read(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
    let f = Flags::parse(args, &["outline", "stdin"]);
    let level = f.int("level", 1);
    if !(0..=2).contains(&level) {
        return Err(usage("--level は 0・1・2"));
    }
    let t = load(cwd)?;
    if f.pos.is_empty() {
        return Err(usage("読むファイルを指定する"));
    }
    let mut docs = Vec::new();
    for n in &f.pos {
        let p = t.resolve(n, cwd).map_err(fail)?;
        docs.push(
            t.get(&p)
                .ok_or_else(|| fail(format!("{p} はファイルでない")))?,
        );
    }
    let opt = ReadOpt {
        level,
        outline: f.on("outline"),
        sections: split_comma(f.str("section").unwrap_or("")),
    };
    let multi = docs.len() > 1;
    for (i, d) in docs.iter().enumerate() {
        if i > 0 && (opt.level != 0 || opt.outline) {
            line(o, "");
        }
        read_doc(o, &t, d, &opt, multi);
    }
    Ok(())
}

fn header(o: &mut Out, p: &str) {
    line(o, &format!("==> {p} <=="));
}

fn read_doc(o: &mut Out, t: &Tree, d: &Doc, opt: &ReadOpt, multi: bool) {
    let path = d.info.path.as_str();
    if opt.outline {
        if multi {
            header(o, path);
        }
        if !d.is_md {
            line(o, &format!("（静的ファイル: {path}）"));
            return;
        }
        if let Some(n) = d.fm_lines() {
            line(o, &format!("frontmatter（{n} 行）"));
        }
        for h in &d.headings {
            line(
                o,
                &format!("{} {}（{} 行）", "#".repeat(h.level), h.text, h.lines()),
            );
        }
    } else if !opt.sections.is_empty() {
        if multi {
            header(o, path);
        }
        let mut first = true;
        for n in &opt.sections {
            let Some(lines) = d.section(n) else {
                o.stderr
                    .push_str(&format!("（{path} に節「{n}」は無い）\n"));
                continue;
            };
            if !first {
                line(o, "");
            }
            first = false;
            line(o, &lines.join("\n"));
        }
    } else if opt.level == 0 {
        for l in summary(d) {
            line(o, &l);
        }
    } else if opt.level == 1 {
        if multi {
            header(o, path);
        }
        content(o, d);
    } else {
        header(o, path);
        content(o, d);
        let linked = t.linked_docs(d);
        if linked.is_empty() {
            return;
        }
        line(o, "");
        line(o, "--- 指す先 ---");
        for ld in &linked {
            for l in summary(ld) {
                line(o, &l);
            }
        }
        for ld in &linked {
            line(o, "");
            header(o, &ld.info.path);
            content(o, ld);
        }
    }
}

fn content(o: &mut Out, d: &Doc) {
    if !d.is_md {
        line(o, &format!("（静的ファイル: {}）", d.info.path));
        return;
    }
    let c = d.content();
    o.stdout.push_str(&c);
    if !c.ends_with('\n') {
        o.stdout.push('\n');
    }
}

fn kind_of(d: &Doc) -> &'static str {
    d.info.kind.map_or("?", Kind::as_str)
}

/// 段 0 の概要（1 件 1〜3 行）。
fn summary(d: &Doc) -> Vec<String> {
    let mut tags = vec![kind_of(d).to_string()];
    if !d.info.project.is_empty() {
        tags.push(d.info.project.clone());
    }
    if d.info.archived {
        tags.push("archived".into());
    }
    let task = d.info.kind == Some(Kind::Task);
    if task {
        let s = d.status();
        if !s.is_empty() {
            tags.push(s);
        }
        if d.needs_human() {
            tags.push("needs_human".into());
        }
    }
    let mut lines = vec![format!("{}  [{}]", d.info.path, tags.join(" · "))];
    if !d.is_md {
        return lines;
    }
    if task {
        lines.push(format!("  {}", d.title()));
        let now = d.now();
        if !now.is_empty() {
            lines.push(format!("  いま: {now}"));
        }
        return lines;
    }
    let desc = d.description();
    if !desc.is_empty() {
        lines.push(format!("  {desc}"));
    } else if !d.title().is_empty() {
        lines.push(format!("  {}", d.title()));
    }
    lines
}

// graph

fn graph(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
    let f = Flags::parse(args, &["in", "out", "all"]);
    let (is_in, is_out) = (f.on("in"), f.on("out"));
    if is_in && is_out {
        return Err(usage("--in と --out はどちらか（両方なら付けない）"));
    }
    let level = f.int("level", -1);
    if level > 2 {
        return Err(usage("--level は 0・1・2"));
    }
    let via = split_comma(f.str("via").unwrap_or(""));
    let format = f.str("format").unwrap_or("");
    let t = load(cwd)?;
    if f.on("all") {
        if !f.pos.is_empty() {
            return Err(usage("--all にパスは付けない"));
        }
        match format {
            "dot" => dot(o, &edges_via(t.edges(), &via)),
            "" | "edges" => panic!("graph --all の表は R4 で比べる"),
            _ => return Err(usage("--all の --format は edges か dot")),
        }
        return Ok(());
    }
    if f.pos.len() != 1 {
        return Err(usage("パスを 1 つ指定する"));
    }
    let start = t.resolve(&f.pos[0], cwd).map_err(fail)?;
    if t.get(&start).is_none() {
        return Err(fail(format!("{start} はファイルでない")));
    }
    let depth = match f.str("depth").unwrap_or("1") {
        "all" => None,
        s => match s.parse::<usize>() {
            Ok(n) if n >= 1 => Some(n),
            _ => return Err(usage("--depth は 1 以上の数か all")),
        },
    };
    let dirs: &[Direction] = if is_in {
        &[Direction::In]
    } else if is_out {
        &[Direction::Out]
    } else {
        &[Direction::Out, Direction::In]
    };
    let w = t.walk(&start, &via, dirs, depth);
    match format {
        "" | "tree" => {
            line(o, &node_line(&t, &start));
            tree_lines(o, &t, &w);
        }
        "paths" => {
            for p in &w.order {
                line(o, p);
            }
        }
        "dot" => dot(o, &w.edges),
        _ => return Err(usage("--format は tree・paths・dot")),
    }
    if level >= 0 && !w.order.is_empty() {
        line(o, "");
        for (i, p) in w.order.iter().enumerate() {
            if i > 0 && level > 0 {
                line(o, "");
            }
            read_doc(o, &t, t.get(p).unwrap(), &ReadOpt::level(level), true);
        }
    }
    Ok(())
}

fn tree_lines(o: &mut Out, t: &Tree, w: &Walk) {
    for s in &w.steps {
        let indent = "  ".repeat(s.depth);
        let arrow = match s.dir {
            Direction::Out => "→",
            Direction::In => "←",
        };
        if s.seen {
            line(o, &format!("{indent}{arrow} {}  {}（既出）", s.key, s.node));
        } else {
            line(
                o,
                &format!("{indent}{arrow} {}  {}", s.key, node_line(t, &s.node)),
            );
        }
    }
}

/// パス、種類、description（無ければ見出し）。
fn node_line(t: &Tree, p: &str) -> String {
    let Some(d) = t.get(p) else {
        return p.to_string();
    };
    let mut s = format!("{p}  [{}", kind_of(d));
    let st = d.status();
    if d.info.kind == Some(Kind::Task) && !st.is_empty() {
        s += &format!(" · {st}");
    }
    s.push(']');
    let desc = d.description();
    if !desc.is_empty() {
        s += &format!("  {desc}");
    } else if !d.title().is_empty() {
        s += &format!("  {}", d.title());
    }
    s
}

fn dot(o: &mut Out, edges: &[Edge]) {
    line(o, "digraph desk {");
    line(o, "  rankdir=LR;");
    line(o, "  node [shape=box, fontsize=10];");
    let mut seen = HashSet::new();
    for e in edges {
        if seen.insert(e) {
            line(
                o,
                &format!(
                    "  {} -> {} [label={}];",
                    go_quote(&e.from),
                    go_quote(&e.to),
                    go_quote(&e.key)
                ),
            );
        }
    }
    line(o, "}");
}

// check

fn check(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
    let f = Flags::parse(args, &["fix", "rules", "json"]);
    if !f.pos.is_empty() {
        return Err(usage(
            "check に引数は付けない（絞るなら --project か --name）",
        ));
    }
    if f.on("rules") {
        o.stdout = describe();
        return Ok(());
    }
    let t = load(cwd)?;
    let target = match f.str("name").filter(|s| !s.is_empty()) {
        Some(n) => Some(t.resolve(n, cwd).map_err(fail)?),
        None => None,
    };
    let project = f.str("project").filter(|s| !s.is_empty());
    let filter = |t: &Tree| -> Vec<Finding> {
        t.check()
            .into_iter()
            .filter(|x| project.is_none_or(|p| x.in_project(p)))
            .filter(|x| target.as_deref().is_none_or(|p| x.concerns(p)))
            .collect()
    };
    #[allow(unused_mut)]
    let mut found = filter(&t);
    if f.on("fix") {
        #[cfg(feature = "write")]
        {
            let changed = desk_core::write::FixPlan::new(&found)
                .apply(&t)
                .map_err(fail)?;
            if !changed.is_empty() {
                found = filter(&Tree::load(t.root()).map_err(fail)?);
            }
            if !f.on("json") && !changed.is_empty() {
                line(o, "直した:");
                for p in &changed {
                    line(o, &format!("  {p}"));
                }
                line(
                    o,
                    &format!(
                        "コミットするとき: git -C {} add {}",
                        display_path(t.root()),
                        changed.join(" ")
                    ),
                );
                if !found.is_empty() {
                    line(o, "");
                }
            }
        }
        #[cfg(not(feature = "write"))]
        panic!("check --fix は feature write で比べる");
    }
    if f.on("json") {
        o.stdout += &(serde_json::to_string_pretty(&found).unwrap() + "\n");
    } else {
        assert!(found.is_empty(), "check の表は R4 で比べる");
        o.stderr.push_str("（形に合っている）\n");
    }
    if !found.is_empty() {
        return Err(Fail {
            code: 1,
            msg: String::new(),
        });
    }
    Ok(())
}

/// ホームの下なら ~/ からのパスにする（Go の displayPath）。
fn display_path(p: &Path) -> String {
    if let Some(home) = std::env::home_dir()
        && let Ok(rel) = p.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }
    p.display().to_string()
}

// 書く側

#[cfg(feature = "write")]
mod write {
    use std::path::Path;

    use desk_core::Tree;
    use desk_core::write::{Error, MovePlan, NewRequest, absolute, init as core_init};

    use super::{Fail, Flags, Out, display_path, fail, line, load, today, usage};

    fn err(e: Error) -> Fail {
        if e.is_usage() { usage(e) } else { fail(e) }
    }

    pub fn init(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
        let f = Flags::parse(args, &[]);
        let dir = match &f.pos[..] {
            [] => ".",
            [d] => d.as_str(),
            _ => return Err(usage("desk init に渡せるパスは 1 つ")),
        };
        let root = absolute(cwd, dir);
        let r = core_init(&root).map_err(err)?;
        if r.already {
            line(
                o,
                &format!("{} はもうツリーのルート（作るものは無い）", root.display()),
            );
            return Ok(());
        }
        line(o, &format!("{} を作った:", root.display()));
        for c in &r.created {
            line(o, &format!("  {c}"));
        }
        if r.committed {
            line(o, "最初のコミットを作った");
        } else if !r.written.is_empty() {
            line(
                o,
                &format!(
                    "コミットはしていない: git -C {} add {}",
                    display_path(&root),
                    r.written.join(" ")
                ),
            );
        }
        line(o, "desk はこのディレクトリかその下で実行する");
        Ok(())
    }

    pub fn new(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
        let f = Flags::parse(args, &["needs-human"]);
        let [kind, slug] = &f.pos[..] else {
            panic!("使い方の文言は R4 で比べる");
        };
        let t = load(cwd)?;
        let today = today();
        let req = NewRequest {
            kind,
            slug,
            project: f.str("project"),
            parent: f.str("parent"),
            needs_human: f.on("needs-human"),
            title: f.str("title"),
            today: &today,
        };
        let plan = t.plan_new(&req, cwd).map_err(err)?;
        let p = plan.apply(&t).map_err(err)?;
        line(o, &p.display().to_string());
        Ok(())
    }

    pub fn archive(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
        let f = Flags::parse(args, &["dry-run"]);
        let status = f.str("status").filter(|s| !s.is_empty());
        if status.is_some_and(|s| s != "done" && s != "dropped") {
            return Err(usage("--status は done か dropped"));
        }
        let t = load(cwd)?;
        let dry = f.on("dry-run");
        if let Some(name) = f.str("project").filter(|s| !s.is_empty()) {
            if !f.pos.is_empty() || status.is_some() {
                return Err(usage("--project にはパスも --status も付けない"));
            }
            let plan = t.plan_archive_project(name).map_err(err)?;
            return perform(o, &t, &plan, dry);
        }
        assert!(!f.pos.is_empty(), "使い方の文言は R4 で比べる");
        let srcs: Vec<&str> = f.pos.iter().map(String::as_str).collect();
        let a = t.plan_archive(&srcs, status, cwd);
        for p in &a.already {
            line(o, &format!("既に archive/ にある: {p}"));
        }
        match a.plan.map_err(err)? {
            Some(plan) => perform(o, &t, &plan, dry),
            None => Ok(()),
        }
    }

    pub fn mv(o: &mut Out, cwd: &Path, args: &[&str]) -> Result<(), Fail> {
        let f = Flags::parse(args, &["dry-run"]);
        assert!(f.pos.len() >= 2, "使い方の文言は R4 で比べる");
        let t = load(cwd)?;
        let (dest, srcs) = f.pos.split_last().unwrap();
        let srcs: Vec<&str> = srcs.iter().map(String::as_str).collect();
        let plan = t.plan_mv(&srcs, dest, cwd).map_err(err)?;
        perform(o, &t, &plan, f.on("dry-run"))
    }

    /// 計画を出すか（--dry-run）、当てて結果を出す。
    fn perform(o: &mut Out, t: &Tree, plan: &MovePlan, dry: bool) -> Result<(), Fail> {
        if dry {
            line(o, "動かす:");
            for m in &plan.moves {
                line(o, &format!("  {} → {}", m.from, m.to));
            }
            if !plan.edits.is_empty() {
                line(o, "書き換える:");
            }
            for (p, fe) in &plan.edits {
                for e in &fe.edits {
                    line(o, &format!("  {p}:{}  {} → {}", e.line + 1, e.old, e.new));
                }
                for ins in &fe.inserts {
                    line(o, &format!("  {p}  （frontmatter に足す）{ins}"));
                }
            }
            return Ok(());
        }
        let res = plan.apply(t).map_err(err)?;
        line(o, "動かした:");
        for m in &res.moved {
            line(o, &format!("  {} → {}", m.from, m.to));
        }
        if !res.rewritten.is_empty() {
            line(o, "書き換えた（リンク・status）:");
            for p in &res.rewritten {
                line(o, &format!("  {p}"));
            }
        }
        line(
            o,
            &format!(
                "コミットするとき: git -C {} add {}",
                display_path(t.root()),
                res.add_paths().join(" ")
            ),
        );
        Ok(())
    }
}
