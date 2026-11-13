//! `desk read`: 段（0 概要・1 全体・2 本文が指す先まで）、outline、section。

use std::io::{self, BufRead, Write};
use std::path::Path;

use desk_core::{Doc, Kind, Tree};

use crate::{Res, fail, line, list, load, note, put, usage};

#[derive(Debug, clap::Args)]
pub struct ReadArgs {
    /// 読むファイル（パスか、1 件に決まる名前）。`-` は標準入力の 1 行を 1 つとして読む（desk ls --paths などの出力）
    #[arg(value_name = "パス", required = true)]
    paths: Vec<String>,
    /// 0: 概要だけ / 1: 全体 / 2: 本文が指す .md まで
    #[arg(long, default_value_t = 1, value_name = "0|1|2", value_parser = clap::value_parser!(i64).range(0..=2))]
    level: i64,
    /// 見出しと各節の行数だけ
    #[arg(long, conflicts_with = "sections")]
    outline: bool,
    /// その節だけ（カンマ区切りか、繰り返して複数）
    #[arg(long = "section", value_name = "見出し", value_delimiter = ',')]
    sections: Vec<String>,
}

pub fn read(out: &mut dyn Write, cwd: &Path, a: ReadArgs) -> Res {
    let t = load(cwd)?;
    let mut names = Vec::new();
    for p in a.paths {
        if p == "-" {
            for l in io::stdin().lock().lines() {
                let l = l.map_err(|e| fail(format!("標準入力を読めない: {e}")))?;
                let l = l.trim();
                if !l.is_empty() {
                    names.push(l.to_string());
                }
            }
        } else {
            names.push(p);
        }
    }
    if names.is_empty() {
        return Err(usage("読むファイルが無い（標準入力が空）"));
    }
    let mut docs = Vec::new();
    for n in &names {
        let p = t.resolve(n, cwd).map_err(fail)?;
        docs.push(
            t.get(&p)
                .ok_or_else(|| fail(format!("{p} はファイルでない")))?,
        );
    }
    let opt = ReadOpt {
        level: a.level,
        outline: a.outline,
        sections: list(a.sections),
    };
    let multi = docs.len() > 1;
    for (i, d) in docs.iter().enumerate() {
        if i > 0 && (opt.level != 0 || opt.outline) {
            line(out, "")?;
        }
        read_doc(out, &t, d, &opt, multi)?;
    }
    Ok(())
}

/// どう読むか。
pub struct ReadOpt {
    pub level: i64,
    pub outline: bool,
    pub sections: Vec<String>,
}

impl ReadOpt {
    /// その段で全体を読む（ls・graph の `--level`）。
    pub fn level(level: i64) -> ReadOpt {
        ReadOpt {
            level,
            outline: false,
            sections: Vec::new(),
        }
    }
}

fn header(out: &mut dyn Write, p: &str) -> io::Result<()> {
    line(out, &format!("==> {p} <=="))
}

/// 1 つのファイルを読む。multi なら（段 2 でなくても）頭に `==> パス <==` を付ける。
pub fn read_doc(
    out: &mut dyn Write,
    t: &Tree,
    d: &Doc,
    opt: &ReadOpt,
    multi: bool,
) -> io::Result<()> {
    let path = d.info.path.as_str();
    if opt.outline {
        if multi {
            header(out, path)?;
        }
        if !d.is_md {
            line(out, &format!("（静的ファイル: {path}）"))?;
            return Ok(());
        }
        if let Some(n) = d.fm_lines() {
            line(out, &format!("frontmatter（{n} 行）"))?;
        }
        for h in &d.headings {
            line(
                out,
                &format!("{} {}（{} 行）", "#".repeat(h.level), h.text, h.lines()),
            )?;
        }
    } else if !opt.sections.is_empty() {
        if multi {
            header(out, path)?;
        }
        let mut first = true;
        for n in &opt.sections {
            let Some(lines) = d.section(n) else {
                note(&format!("（{path} に節「{n}」は無い）"));
                continue;
            };
            if !first {
                line(out, "")?;
            }
            first = false;
            line(out, &lines.join("\n"))?;
        }
    } else if opt.level == 0 {
        for l in summary(d) {
            line(out, &l)?;
        }
    } else if opt.level == 1 {
        if multi {
            header(out, path)?;
        }
        content(out, d)?;
    } else {
        header(out, path)?;
        content(out, d)?;
        let linked = t.linked_docs(d);
        if linked.is_empty() {
            return Ok(());
        }
        line(out, "")?;
        line(out, "--- 指す先 ---")?;
        for ld in &linked {
            for l in summary(ld) {
                line(out, &l)?;
            }
        }
        for ld in &linked {
            line(out, "")?;
            header(out, &ld.info.path)?;
            content(out, ld)?;
        }
    }
    Ok(())
}

fn content(out: &mut dyn Write, d: &Doc) -> io::Result<()> {
    if !d.is_md {
        line(out, &format!("（静的ファイル: {}）", d.info.path))?;
        return Ok(());
    }
    let c = d.content();
    put(out, &c)?;
    if !c.ends_with('\n') {
        line(out, "")?;
    }
    Ok(())
}

pub fn kind_of(d: &Doc) -> &'static str {
    d.info.kind.map_or("?", Kind::as_str)
}

/// 段 0 の概要（1 件 1〜3 行）。
pub fn summary(d: &Doc) -> Vec<String> {
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
