//! `desk new`: 形どおりのファイルを 1 つ作る。

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use relative_path::RelativePath;

use super::Error;
use crate::link::link_text;
use crate::path::{project_dir, split_dated};
use crate::rules::{ARCHIVE_DIR, HUMAN_TASK_SECTIONS, Kind, rule_of};
use crate::tree::Tree;

static SLUG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap());

/// `desk new` に渡すもの。空の文字列は渡していないのと同じ。
#[derive(Debug, Clone, Default)]
pub struct NewRequest<'a> {
    /// 種類の名前（`project` も）
    pub kind: &'a str,
    pub slug: &'a str,
    pub project: Option<&'a str>,
    /// 親タスク（task の parent、artifact の tasks）。パスか名前
    pub parent: Option<&'a str>,
    pub needs_human: bool,
    /// 見出し（省略は slug）
    pub title: Option<&'a str>,
    /// 日付の付く種類の名前に使う今日（YYYY-MM-DD）
    pub today: &'a str,
}

/// 作るファイル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPlan {
    /// ルートからのパス
    pub path: String,
    pub content: String,
}

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.is_empty())
}

impl Tree {
    /// 作るファイルを決める。cwd は `--parent` の名前を解決するときの作業ディレクトリ。
    pub fn plan_new(&self, req: &NewRequest<'_>, cwd: &Path) -> Result<NewPlan, Error> {
        let mut project = non_empty(req.project).map(str::to_string);
        let parent = non_empty(req.parent);
        let slug = req.slug;

        if req.kind == "project" {
            if !SLUG_RE.is_match(slug) {
                return Err(Error::usage(format!(
                    "プロジェクトの名前は英小文字・数字・ハイフン（{slug}）"
                )));
            }
            let pd = project_dir(slug, false);
            if self.exists(&pd) {
                return Err(Error::failed(format!(
                    "既にある: {}",
                    self.abs(&pd).display()
                )));
            }
            let archived = project_dir(slug, true);
            if self.exists(&archived) {
                return Err(Error::failed(format!(
                    "archive/ に同じ名前のプロジェクトがある: {}",
                    self.abs(&archived).display()
                )));
            }
            return Ok(NewPlan {
                path: format!("{pd}/README.md"),
                content: format!("# {slug}\n"),
            });
        }

        let Some((kind, rule)) = Kind::from_name(req.kind).and_then(|k| Some((k, rule_of(k)?)))
        else {
            return Err(Error::usage(format!(
                "知らない種類 {:?}（task artifact inbox library principle knowledge material wiki project）",
                req.kind
            )));
        };
        if req.needs_human && kind != Kind::Task {
            return Err(Error::usage("--needs-human は task だけ"));
        }
        if parent.is_some() && kind != Kind::Task && kind != Kind::Artifact {
            return Err(Error::usage(
                "--parent は task（parent）と artifact（tasks）だけ",
            ));
        }

        // 名前
        let mut slug = slug.to_string();
        let mut date = String::new();
        if rule.dated {
            date = req.today.to_string();
            let name = format!("{slug}.md");
            if let Some((d, s)) = split_dated(&name) {
                (date, slug) = (d.to_string(), s.to_string());
            }
        }
        check_slug(kind, &slug)?;
        let mut name = format!("{slug}.md");
        if !date.is_empty() {
            name = format!("{date}-{name}");
        }

        // 親
        let mut parent_doc = None;
        if let Some(arg) = parent {
            let p = self
                .resolve(arg, cwd)
                .map_err(|e| Error::failed(e.to_string()))?;
            let d = self
                .get(&p)
                .filter(|d| d.info.kind == Some(Kind::Task))
                .ok_or_else(|| Error::usage(format!("{p} はタスクでない")))?;
            if project.is_none() {
                project = Some(d.info.project.clone());
                if d.info.project_archived {
                    return Err(Error::usage(format!(
                        "{p} は archive/ に移ったプロジェクトのタスク"
                    )));
                }
            }
            parent_doc = Some(d);
        }
        let project = project.filter(|p| !p.is_empty());

        // 置き場所
        let place = match &project {
            None if rule.in_root => rule.dir.to_string(),
            None => {
                return Err(Error::usage(format!(
                    "{} はプロジェクトに置く（--project <名前>。どこにも属さないなら misc）",
                    kind.as_str()
                )));
            }
            Some(p) => {
                let pd = project_dir(p, false);
                if p.contains('/') || !self.abs(&pd).is_dir() {
                    return Err(Error::usage(format!(
                        "プロジェクト {p} が無い（desk new project {p} で作る）"
                    )));
                }
                format!("{pd}/{}", rule.dir)
            }
        };
        let rel = format!("{place}/{name}");

        // 同じ種類に同じ名前（archive/ の中も）
        let mut same = vec![rel.clone()];
        if let (true, Some(p)) = (rule.archivable, &project) {
            same.push(format!(
                "{}/{ARCHIVE_DIR}/{}/{name}",
                project_dir(p, false),
                rule.dir
            ));
        }
        for s in &same {
            if self.exists(s) {
                return Err(Error::failed(format!(
                    "既にある: {}",
                    self.abs(s).display()
                )));
            }
        }

        let heading = non_empty(req.title)
            .unwrap_or_else(|| RelativePath::new(&slug).file_name().unwrap_or(&slug));
        let mut sections = rule.sections;
        let parent_link = parent_doc.map(|d| link_text(&rel, &d.info.path, ""));
        let fm: Option<Vec<String>> = match kind {
            Kind::Task => {
                let mut fm = vec![
                    "status: todo".to_string(),
                    format!("needs_human: {}", req.needs_human),
                ];
                if let Some(l) = parent_link {
                    fm.push(format!("parent: {l}"));
                }
                if req.needs_human {
                    sections = HUMAN_TASK_SECTIONS;
                }
                Some(fm)
            }
            Kind::Artifact => Some(vec![format!(
                "tasks: [{}]",
                parent_link.unwrap_or_default()
            )]),
            Kind::Wiki | Kind::Principle => Some(vec!["description:".to_string()]),
            Kind::Knowledge => Some(vec![
                "description:".to_string(),
                "principle:".to_string(),
                "sources: []".to_string(),
            ]),
            Kind::Material => Some(vec!["knowledge: []".to_string()]),
            _ => None,
        };
        Ok(NewPlan {
            path: rel,
            content: render(fm.as_deref(), heading, sections),
        })
    }
}

impl NewPlan {
    /// ファイルを作る。もうあれば何もせずに止まる。作ったファイルの絶対パスを返す。
    pub fn apply(&self, tree: &Tree) -> Result<PathBuf, Error> {
        let p = tree.abs(&self.path);
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?;
        }
        let mut f = match OpenOptions::new().write(true).create_new(true).open(&p) {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                return Err(Error::failed(format!("既にある: {}", p.display())));
            }
            Err(e) => return Err(e.into()),
        };
        f.write_all(self.content.as_bytes())?;
        Ok(p)
    }
}

fn check_slug(kind: Kind, slug: &str) -> Result<(), Error> {
    let segs: Vec<&str> = if kind == Kind::Wiki {
        slug.split('/').collect()
    } else {
        vec![slug]
    };
    let last = segs.len() - 1;
    for (i, s) in segs.iter().enumerate() {
        if kind == Kind::Wiki && i == last && *s == "README" {
            continue;
        }
        if !SLUG_RE.is_match(s) {
            return Err(Error::usage(format!(
                "slug は英小文字・数字・ハイフン（{slug}）"
            )));
        }
    }
    Ok(())
}

fn render(fm: Option<&[String]>, title: &str, sections: &[&str]) -> String {
    let mut b = String::new();
    if let Some(fm) = fm {
        b.push_str("---\n");
        for l in fm {
            b.push_str(l);
            b.push('\n');
        }
        b.push_str("---\n\n");
    }
    b.push_str("# ");
    b.push_str(title);
    b.push('\n');
    for s in sections {
        b.push_str("\n## ");
        b.push_str(s);
        b.push('\n');
    }
    b
}
