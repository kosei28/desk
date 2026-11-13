//! 確かめる: `desk check`。形に合っているかと、ツリーの中のずれ。
//!
//! 見つけたものに、直し方の文言と、`--fix` で直せるならその直し（[`Remedy`]）を持たせる。直しを当てるのは書く側
//! （feature `write`）。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use relative_path::RelativePath;
use serde::{Serialize, Serializer};

use crate::doc::Doc;
use crate::link::{LinkForm, link_text};
use crate::path::{archive_path, classify, is_date, name_problem, unarchive_path, unique_scope};
use crate::rules::{
    ARCHIVE_DIR, HUMAN_TASK_SECTIONS, KeyRule, Kind, LOG_SECTION, PROJECTS_DIR, ROOT_RESERVED,
    TASK_SECTIONS, project_entries, rule_by_dir, rule_of,
};
use crate::tree::Tree;

/// 何を確かめたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Dir,
    Name,
    Frontmatter,
    Link,
    Shape,
    Drift,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Dir => "ディレクトリ",
            Category::Name => "名前",
            Category::Frontmatter => "frontmatter",
            Category::Link => "リンク",
            Category::Shape => "構成",
            Category::Drift => "ずれ",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Category {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

/// check で見つかったこと 1 つ。`check --json` の 1 件の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// ルートからのパス（ディレクトリは / で終わる）
    pub path: String,
    /// 1 始まり。ファイル全体なら 0
    #[serde(skip_serializing_if = "is_zero")]
    pub line: usize,
    pub category: Category,
    pub problem: String,
    pub fix: String,
    pub fixable: bool,
    /// `--name` で一緒に挙げるファイル
    #[serde(skip)]
    pub related: Vec<String>,
    /// `--fix` で当てる直し（fixable のときだけ）
    #[serde(skip)]
    pub remedy: Option<Remedy>,
}

/// `--fix` で当てる、機械的に 1 通りに決まる直し。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remedy {
    /// frontmatter の最初に行を足す（frontmatter が無ければ作る）
    Insert(Vec<String>),
    /// 行の中のリンクの先を書き換える
    Relink {
        /// ファイルの行（0 始まり）
        line: usize,
        /// 行の中のバイト位置
        start: usize,
        old: String,
        new: String,
    },
    /// archive/ の中か外へ動かす（`desk archive` と同じくリンクも付け替える）
    Move { from: String, to: String },
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl Finding {
    fn new(path: &str, line: usize, category: Category, problem: String, fix: &str) -> Finding {
        Finding {
            path: path.to_string(),
            line,
            category,
            problem,
            fix: fix.to_string(),
            fixable: false,
            related: Vec::new(),
            remedy: None,
        }
    }

    fn related(mut self, related: Vec<String>) -> Finding {
        self.related = related;
        self
    }

    fn remedy(mut self, remedy: Remedy) -> Finding {
        self.fixable = true;
        self.remedy = Some(remedy);
        self
    }

    /// そのプロジェクトのものか（`check --project`）。
    pub fn in_project(&self, project: &str) -> bool {
        classify(self.path.trim_end_matches('/')).project == project
    }

    /// そのファイルの分か、そのファイルが関係するものか（`check --name`）。
    pub fn concerns(&self, path: &str) -> bool {
        self.path == path || self.related.iter().any(|r| r == path)
    }
}

impl Tree {
    /// ツリー全体を確かめる。パス、行の順。
    pub fn check(&self) -> Vec<Finding> {
        let mut c = Checker {
            t: self,
            out: Vec::new(),
            details: knowledge_details(self),
        };
        let bad = c.check_dirs();
        for d in self.docs() {
            c.check_place(d, &bad);
            if !d.is_md {
                continue;
            }
            c.check_fm(d);
            c.check_links(d);
            c.check_shape(d);
        }
        c.check_names();
        c.check_assets();
        c.check_drift();
        c.out
            .sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
        c.out
    }
}

struct Checker<'t> {
    t: &'t Tree,
    out: Vec<Finding>,
    /// ナレッジの詳細（ほかのナレッジの本文から指されているナレッジ）
    details: HashSet<String>,
}

/// . で始まらない名前を、名前の順に（ディレクトリかどうかと一緒に）。
fn list_dir(abs: &Path) -> Vec<(String, bool)> {
    let Ok(rd) = fs::read_dir(abs) else {
        return Vec::new();
    };
    let mut out: Vec<(String, bool)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            (!name.starts_with('.')).then_some((name, is_dir))
        })
        .collect();
    out.sort();
    out
}

fn finished(status: &str) -> bool {
    status == "done" || status == "dropped"
}

/// まだ終わっていない status。
fn is_open(status: &str) -> bool {
    matches!(status, "backlog" | "todo" | "inprogress")
}

/// 足す行（`name:`、配列なら `name: []`）。
pub fn key_line(k: &KeyRule) -> String {
    if k.list {
        format!("{}: []", k.name)
    } else {
        format!("{}:", k.name)
    }
}

fn fix_text(ins: &[String]) -> String {
    format!("--fix で足す（{}）", ins.join("、"))
}

/// ほかのナレッジの本文から指されているナレッジ（詳細）。
fn knowledge_details(t: &Tree) -> HashSet<String> {
    let mut out = HashSet::new();
    for d in t.docs() {
        if d.info.kind != Some(Kind::Knowledge) {
            continue;
        }
        for l in &d.links {
            if l.key != "body" || l.form != LinkForm::Relative || l.target == d.info.path {
                continue;
            }
            if t.get(&l.target)
                .is_some_and(|to| to.info.kind == Some(Kind::Knowledge))
            {
                out.insert(l.target.clone());
            }
        }
    }
    out
}

/// key のリンクのうち相対パスのものの先。
fn link_targets<'d>(d: &'d Doc, key: &str) -> impl Iterator<Item = &'d str> {
    d.links
        .iter()
        .filter(move |l| l.key == key && l.form == LinkForm::Relative)
        .map(|l| l.target.as_str())
}

impl Checker<'_> {
    fn add(&mut self, f: Finding) {
        self.out.push(f);
    }

    // ディレクトリ

    /// ディレクトリの形を確かめ、決まりの外のディレクトリ（中のファイルは個別に挙げない）を返す。
    fn check_dirs(&mut self) -> Vec<String> {
        let t = self.t;
        let mut bad: Vec<(String, &str)> = Vec::new();
        for (name, is_dir) in list_dir(t.root()) {
            if is_dir && !ROOT_RESERVED.contains(&name.as_str()) {
                // ファイルは classify が挙げる
                bad.push((
                    name,
                    "ルートに置けるのは決まった名前だけ（プロジェクトは projects/ の下に置く）",
                ));
            }
        }
        let check_project = |bad: &mut Vec<(String, &str)>, d: &str| {
            for (name, is_dir) in list_dir(&t.abs(d)) {
                if is_dir && !project_entries().contains(&name.as_str()) {
                    bad.push((format!("{d}/{name}"), "プロジェクトに置けないディレクトリ"));
                }
            }
            for (name, is_dir) in list_dir(&t.abs(&format!("{d}/{ARCHIVE_DIR}"))) {
                if is_dir && !rule_by_dir(&name).is_some_and(|r| r.archivable) {
                    bad.push((
                        format!("{d}/{ARCHIVE_DIR}/{name}"),
                        "プロジェクトの archive/ に置くのは tasks/ と artifacts/ だけ",
                    ));
                }
            }
        };
        for (name, is_dir) in list_dir(&t.abs(PROJECTS_DIR)) {
            if is_dir {
                check_project(&mut bad, &format!("{PROJECTS_DIR}/{name}"));
            }
        }
        for (name, is_dir) in list_dir(&t.abs(ARCHIVE_DIR)) {
            if is_dir && name != PROJECTS_DIR {
                bad.push((
                    format!("{ARCHIVE_DIR}/{name}"),
                    "ルートの archive/ には projects/<プロジェクト>/ だけを置く",
                ));
            }
        }
        for (name, is_dir) in list_dir(&t.abs(&format!("{ARCHIVE_DIR}/{PROJECTS_DIR}"))) {
            if is_dir {
                check_project(&mut bad, &format!("{ARCHIVE_DIR}/{PROJECTS_DIR}/{name}"));
            }
        }
        bad.into_iter()
            .map(|(rel, problem)| {
                self.add(Finding::new(
                    &format!("{rel}/"),
                    0,
                    Category::Dir,
                    problem.to_string(),
                    "中身を決まりの場所に desk mv で動かす",
                ));
                rel
            })
            .collect()
    }

    /// ファイルの置き場所（classify の結果）を挙げる。
    fn check_place(&mut self, d: &Doc, bad: &[String]) {
        let Some(problem) = &d.info.problem else {
            return;
        };
        if bad
            .iter()
            .any(|b| d.info.path.starts_with(&format!("{b}/")))
        {
            return;
        }
        self.add(Finding::new(
            &d.info.path,
            0,
            Category::Dir,
            problem.clone(),
            "決まりの場所に desk mv で動かす",
        ));
    }

    // 名前

    fn check_names(&mut self) {
        let mut groups: HashMap<String, Vec<&Doc>> = HashMap::new();
        let mut keys = Vec::new();
        for d in self.t.docs() {
            if d.info.problem.is_some() {
                continue;
            }
            if let Some(p) = name_problem(&d.info) {
                self.add(Finding::new(
                    &d.info.path,
                    0,
                    Category::Name,
                    p,
                    "desk mv で名前を変える（日付は作った日）",
                ));
            }
            let key = match d.info.kind {
                None | Some(Kind::Wiki | Kind::Readme) => continue, // wiki/ は入れ子でパスが識別になる
                Some(_) if d.info.asset => {
                    let p = unarchive_path(&d.info);
                    let dir = RelativePath::new(&p)
                        .parent()
                        .map_or("", RelativePath::as_str);
                    format!("{dir}|{}", d.info.name())
                }
                Some(_) => format!(
                    "{}|{}",
                    unique_scope(&d.info).unwrap_or_default(),
                    d.info.name()
                ),
            };
            let g = groups.entry(key.clone()).or_default();
            if g.is_empty() {
                keys.push(key);
            }
            g.push(d);
        }
        for k in keys {
            let g = &groups[&k];
            if g.len() < 2 {
                continue;
            }
            let paths: Vec<String> = g.iter().map(|d| d.info.path.clone()).collect();
            for d in g {
                self.out.push(
                    Finding::new(
                        &d.info.path,
                        0,
                        Category::Name,
                        format!("同じ種類の中で名前が重複している（{}）", paths.join("、")),
                        "中身を 1 つに寄せて片方を消すか、desk mv で名前を変える",
                    )
                    .related(paths.clone()),
                );
            }
        }
    }

    // frontmatter

    /// d に当てる frontmatter のキーの決まり。ナレッジの詳細には principle を求めない。
    fn key_rules(&self, d: &Doc, keys: &[KeyRule]) -> Vec<KeyRule> {
        let detail = d.info.kind == Some(Kind::Knowledge) && self.details.contains(&d.info.path);
        keys.iter()
            .map(|&k| {
                if detail && k.name == "principle" {
                    KeyRule {
                        required: false,
                        non_empty: false,
                        ..k
                    }
                } else {
                    k
                }
            })
            .collect()
    }

    fn check_fm(&mut self, d: &Doc) {
        let path = d.info.path.as_str();
        if let Some(e) = &d.fm_error {
            self.add(Finding::new(
                path,
                1,
                Category::Frontmatter,
                e.clone(),
                "手で直す",
            ));
            return;
        }
        let Some(r) = d.info.kind.and_then(rule_of) else {
            return;
        };
        if d.info.problem.is_some() {
            return;
        }
        let keys = self.key_rules(d, r.keys);
        if !d.has_fm {
            if !r.need_fm {
                return;
            }
            let ins: Vec<String> = keys.iter().filter(|k| k.required).map(key_line).collect();
            self.add(
                Finding::new(
                    path,
                    1,
                    Category::Frontmatter,
                    "frontmatter が無い".to_string(),
                    &fix_text(&ins),
                )
                .remedy(Remedy::Insert(ins)),
            );
            for k in keys.iter().filter(|k| k.required && k.non_empty) {
                self.add(Finding::new(
                    path,
                    1,
                    Category::Frontmatter,
                    format!("{} が空", k.name),
                    "中身を書く",
                ));
            }
            return;
        }
        let fm = d.frontmatter.as_ref();
        for k in &keys {
            let Some(field) = fm.and_then(|fm| fm.get(k.name)) else {
                if k.required {
                    let ins = vec![key_line(k)];
                    self.add(
                        Finding::new(
                            path,
                            1,
                            Category::Frontmatter,
                            format!("{} が無い", k.name),
                            &fix_text(&ins),
                        )
                        .remedy(Remedy::Insert(ins)),
                    );
                    if k.non_empty {
                        self.add(Finding::new(
                            path,
                            1,
                            Category::Frontmatter,
                            format!("{} が空", k.name),
                            "中身を書く",
                        ));
                    }
                }
                continue;
            };
            // 値の行（1 始まり）
            let line = field.line + 1;
            let vals = d.list(k.name);
            if k.non_empty && vals.concat().trim().is_empty() {
                self.add(Finding::new(
                    path,
                    line,
                    Category::Frontmatter,
                    format!("{} が空", k.name),
                    "中身を書く",
                ));
                continue;
            }
            for v in &vals {
                if !k.values.is_empty() && !k.values.contains(&v.as_str()) {
                    self.add(Finding::new(
                        path,
                        line,
                        Category::Frontmatter,
                        format!(
                            "{} の値 {v:?} は決まりの外（{}）",
                            k.name,
                            k.values.join("・")
                        ),
                        "手で直す",
                    ));
                }
                if k.date && !is_date(v) {
                    self.add(Finding::new(
                        path,
                        line,
                        Category::Frontmatter,
                        format!("{} の値 {v:?} が日付（YYYY-MM-DD）でない", k.name),
                        "手で直す",
                    ));
                }
            }
        }
    }

    // リンク

    fn link_exists(&self, target: &str) -> bool {
        if target.starts_with("../") || target == ".." {
            return self.t.root().join(target).exists();
        }
        self.t.exists(target)
    }

    fn check_links(&mut self, d: &Doc) {
        let path = d.info.path.as_str();
        for l in &d.links {
            let place = if l.key == "body" { "本文" } else { &l.key };
            let line = l.line + 1;
            match l.form {
                LinkForm::Absolute | LinkForm::Home => self.add(Finding::new(
                    path,
                    line,
                    Category::Link,
                    format!("{place} のリンク {} が相対パスでない", l.raw),
                    "相対パスに書き直す",
                )),
                LinkForm::Wiki => self.add(Finding::new(
                    path,
                    line,
                    Category::Link,
                    format!("{place} のリンク {} が名前だけ", l.raw),
                    "相対パスの markdown のリンクに書き直す",
                )),
                LinkForm::Relative => {
                    let p = l.raw.split_once('#').map_or(l.raw.as_str(), |(p, _)| p);
                    if p.is_empty() || self.link_exists(&l.target) {
                        continue;
                    }
                    let problem = format!("{place} のリンク {} の先が無い", l.raw);
                    let name = RelativePath::new(&l.target).file_name().unwrap_or_default();
                    let cands = self.t.find_by_name(name);
                    let f = if let ([cand], Some(start)) = (&cands[..], l.start) {
                        let to = link_text(path, &cand.info.path, &l.anchor);
                        Finding::new(
                            path,
                            line,
                            Category::Link,
                            problem,
                            &format!(
                                "--fix で {to} に付け替える（同じ名前のファイルがツリーに 1 つだけある）"
                            ),
                        )
                        .remedy(Remedy::Relink {
                            line: l.line,
                            start,
                            old: l.raw.clone(),
                            new: to,
                        })
                    } else if cands.len() > 1 {
                        Finding::new(
                            path,
                            line,
                            Category::Link,
                            problem,
                            "同じ名前のファイルが複数ある。どれを指すか確かめて手で直す",
                        )
                    } else {
                        Finding::new(
                            path,
                            line,
                            Category::Link,
                            problem,
                            "先を探して手で直す（消したならリンクも消す）",
                        )
                    };
                    self.add(f);
                }
                LinkForm::External | LinkForm::Anchor => {}
            }
        }
    }

    /// assets/ の中でどこからも指されていないファイルを挙げる。
    fn check_assets(&mut self) {
        let used: HashSet<&str> = self
            .t
            .docs()
            .iter()
            .flat_map(|d| &d.links)
            .filter(|l| l.form == LinkForm::Relative)
            .map(|l| l.target.as_str())
            .collect();
        for d in self.t.docs() {
            if d.info.asset && d.info.problem.is_none() && !used.contains(d.info.path.as_str()) {
                self.out.push(Finding::new(
                    &d.info.path,
                    0,
                    Category::Link,
                    "assets/ の中のファイルがどこからも指されていない".to_string(),
                    "要らなければ消す。要るなら指す文書からリンクする",
                ));
            }
        }
    }

    // 内容の構成

    fn check_shape(&mut self, d: &Doc) {
        if d.info.kind != Some(Kind::Task) || d.info.problem.is_some() {
            return;
        }
        let want = if d.needs_human() {
            HUMAN_TASK_SECTIONS
        } else {
            TASK_SECTIONS
        };
        let h2: Vec<_> = d.h2().collect();
        for s in want {
            if !h2.iter().any(|h| h.text == *s) {
                self.add(Finding::new(
                    &d.info.path,
                    0,
                    Category::Shape,
                    format!("必須の節「{s}」が無い"),
                    "節を足して書く",
                ));
            }
        }
        for (i, h) in h2.iter().enumerate() {
            if h.text == LOG_SECTION && i != h2.len() - 1 {
                self.add(Finding::new(
                    &d.info.path,
                    h.line + 1,
                    Category::Shape,
                    "ログの節が最後にない".to_string(),
                    "ログの節をいちばん下に移す",
                ));
            }
        }
    }

    // ツリーの中のずれ

    fn check_drift(&mut self) {
        let t = self.t;
        let tasks: Vec<&Doc> = t
            .docs()
            .iter()
            .filter(|d| d.info.kind == Some(Kind::Task) && d.info.problem.is_none() && d.is_md)
            .collect();
        let mut children: HashMap<&str, Vec<&Doc>> = HashMap::new();
        for d in &tasks {
            for p in link_targets(d, "parent") {
                children.entry(p).or_default().push(d);
            }
        }
        for d in &tasks {
            let path = d.info.path.as_str();
            let st = d.status();
            if finished(&st) && !d.info.archived {
                let to = archive_path(&d.info);
                let problem = format!("status が {st} なのに archive/ の外");
                self.add(if t.exists(&to) {
                    Finding::new(
                        path,
                        0,
                        Category::Drift,
                        problem,
                        &format!("{to} が既にある。中身を 1 つに寄せる"),
                    )
                } else {
                    Finding::new(
                        path,
                        0,
                        Category::Drift,
                        problem,
                        "--fix で archive/ に移す（desk archive と同じ）",
                    )
                    .remedy(Remedy::Move {
                        from: path.to_string(),
                        to,
                    })
                });
            } else if is_open(&st) && d.info.archived {
                if d.info.project_archived {
                    self.add(Finding::new(
                        path,
                        0,
                        Category::Drift,
                        format!("archive に移したプロジェクトの中で status が {st}"),
                        "閉じるか、プロジェクトを projects/ に戻す",
                    ));
                } else {
                    let to = unarchive_path(&d.info);
                    let problem = format!("status が {st} なのに archive/ の中");
                    self.add(if t.exists(&to) {
                        Finding::new(
                            path,
                            0,
                            Category::Drift,
                            problem,
                            &format!("{to} が既にある。中身を 1 つに寄せる"),
                        )
                    } else {
                        Finding::new(
                            path,
                            0,
                            Category::Drift,
                            problem,
                            "--fix で archive/ の外に戻す",
                        )
                        .remedy(Remedy::Move {
                            from: path.to_string(),
                            to,
                        })
                    });
                }
            }

            // 子と親
            if let Some(kids) = children.get(path) {
                let all = kids.iter().all(|k| finished(&k.status()));
                if st == "inprogress" && all {
                    let paths = kids.iter().map(|k| k.info.path.clone()).collect();
                    self.add(
                        Finding::new(
                            path,
                            0,
                            Category::Drift,
                            "子タスクが全部終わっているのに inprogress".to_string(),
                            "親を閉じるか、残りを手順に書く",
                        )
                        .related(paths),
                    );
                }
            }
            for p in link_targets(d, "parent") {
                let Some(pd) = t.get(p).filter(|pd| pd.info.path != path) else {
                    continue;
                };
                let ps = pd.status();
                let fix = if finished(&ps) && is_open(&st) {
                    "子を閉じるか、親に戻す"
                } else if st == "inprogress" && (ps == "backlog" || ps == "todo")
                    || st == "todo" && ps == "backlog"
                {
                    "親の status を進めるか、子を戻す"
                } else {
                    continue;
                };
                self.add(
                    Finding::new(
                        path,
                        0,
                        Category::Drift,
                        format!("親タスク {p} が {ps} なのに {st}"),
                        fix,
                    )
                    .related(vec![p.to_string()]),
                );
            }

            // 依存
            for p in link_targets(d, "depends_on") {
                let Some(dd) = t.get(p).filter(|dd| dd.info.path != path) else {
                    continue;
                };
                let ds = dd.status();
                let related = vec![p.to_string()];
                if (st == "inprogress" || st == "done") && ds != "done" {
                    self.add(
                        Finding::new(
                            path,
                            0,
                            Category::Drift,
                            format!("depends_on の {p} が {ds} なのに {st}"),
                            "依存が done になるまで status を戻すか、依存を外す",
                        )
                        .related(related.clone()),
                    );
                }
                if st == "todo" && ds == "backlog" {
                    self.add(
                        Finding::new(
                            path,
                            0,
                            Category::Drift,
                            format!("todo なのに depends_on の {p} が backlog"),
                            "依存を todo 以上にするか、このタスクを backlog に戻す",
                        )
                        .related(related.clone()),
                    );
                }
                if st != "dropped" && ds == "dropped" {
                    self.add(
                        Finding::new(
                            path,
                            0,
                            Category::Drift,
                            format!("depends_on の {p} が dropped"),
                            "依存を外すか差し替える。続けないならこのタスクも dropped にする",
                        )
                        .related(related),
                    );
                }
            }
        }
        self.check_cycles(&tasks, "parent");
        self.check_cycles(&tasks, "depends_on");
    }

    /// key のリンクが自分自身を指すか、循環しているタスクを挙げる。
    fn check_cycles(&mut self, tasks: &[&Doc], key: &str) {
        let mut next: HashMap<&str, Vec<&str>> = HashMap::new();
        for d in tasks {
            for p in link_targets(d, key) {
                if self
                    .t
                    .get(p)
                    .is_some_and(|dd| dd.info.kind == Some(Kind::Task))
                {
                    next.entry(&d.info.path).or_default().push(p);
                }
            }
        }
        for d in tasks {
            let path = d.info.path.as_str();
            for &p in next.get(path).into_iter().flatten() {
                if p == path {
                    self.add(Finding::new(
                        path,
                        0,
                        Category::Drift,
                        format!("{key} が自分自身を指している"),
                        &format!("{key} から自分を外す"),
                    ));
                }
            }
        }
        let order: Vec<&str> = tasks.iter().map(|d| d.info.path.as_str()).collect();
        for scc in sccs(&order, &next) {
            if scc.len() < 2 {
                continue;
            }
            for &p in &scc {
                let others = scc
                    .iter()
                    .filter(|&&q| q != p)
                    .map(|q| q.to_string())
                    .collect();
                self.add(
                    Finding::new(
                        p,
                        0,
                        Category::Drift,
                        format!("{key} が循環している（{}）", scc.join("、")),
                        &format!("どれかの {key} を外す"),
                    )
                    .related(others),
                );
            }
        }
    }
}

/// 強連結成分（Tarjan）。各成分の中はパスの順。
fn sccs<'a>(nodes: &[&'a str], next: &HashMap<&'a str, Vec<&'a str>>) -> Vec<Vec<&'a str>> {
    struct S<'a, 'n> {
        next: &'n HashMap<&'a str, Vec<&'a str>>,
        index: HashMap<&'a str, usize>,
        low: HashMap<&'a str, usize>,
        on: HashSet<&'a str>,
        stack: Vec<&'a str>,
        out: Vec<Vec<&'a str>>,
    }
    impl<'a> S<'a, '_> {
        fn visit(&mut self, v: &'a str) {
            let n = self.index.len();
            self.index.insert(v, n);
            self.low.insert(v, n);
            self.stack.push(v);
            self.on.insert(v);
            for &w in self.next.get(v).into_iter().flatten() {
                if !self.index.contains_key(w) {
                    self.visit(w);
                    let lw = self.low[w];
                    let lv = self.low.get_mut(v).unwrap();
                    *lv = (*lv).min(lw);
                } else if self.on.contains(w) {
                    let iw = self.index[w];
                    let lv = self.low.get_mut(v).unwrap();
                    *lv = (*lv).min(iw);
                }
            }
            if self.low[v] == self.index[v] {
                let mut comp = Vec::new();
                loop {
                    let w = self.stack.pop().unwrap();
                    self.on.remove(w);
                    comp.push(w);
                    if w == v {
                        break;
                    }
                }
                comp.sort();
                self.out.push(comp);
            }
        }
    }
    let mut s = S {
        next,
        index: HashMap::new(),
        low: HashMap::new(),
        on: HashSet::new(),
        stack: Vec::new(),
        out: Vec::new(),
    };
    for &v in nodes {
        if !s.index.contains_key(v) {
            s.visit(v);
        }
    }
    s.out
}
