//! ルートからのパスで決まること（種類、プロジェクト、archive の中か）。

use std::sync::LazyLock;

use regex::Regex;
use relative_path::RelativePath;
use serde::Serialize;

use crate::rules::{ARCHIVE_DIR, Kind, KindRule, PROJECTS_DIR, rule_by_dir, rule_of};

/// ルートからのパスで決まること。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Info {
    /// ルートからのパス（/ 区切り）
    pub path: String,
    /// 分からなければ None
    pub kind: Option<Kind>,
    /// ルートのものは空
    pub project: String,
    /// archive/ の中（プロジェクトの archive/ か、ルートの archive/ のプロジェクト）
    pub archived: bool,
    /// ルートの archive/ に移ったプロジェクトの中
    pub project_archived: bool,
    /// assets/ の中の静的ファイル
    pub asset: bool,
    /// 決まりの外の場所なら、その説明
    pub problem: Option<String>,
}

static DATED_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9]{4}-[0-9]{2}-[0-9]{2})-(.+)\.md$").unwrap());

impl Info {
    /// ファイル名。
    pub fn name(&self) -> &str {
        RelativePath::new(&self.path)
            .file_name()
            .unwrap_or_default()
    }

    /// そのファイルがあるプロジェクトのディレクトリ（projects/<p>、archive に移ったものは
    /// archive/projects/<p>）。ルートのものは空。
    pub fn project_dir(&self) -> String {
        if self.project.is_empty() {
            return String::new();
        }
        project_dir(&self.project, self.project_archived)
    }

    fn set_problem(&mut self, s: impl Into<String>) {
        self.problem = Some(s.into());
    }
}

/// プロジェクトのディレクトリのルートからのパス。
pub fn project_dir(name: &str, archived: bool) -> String {
    if archived {
        format!("{ARCHIVE_DIR}/{PROJECTS_DIR}/{name}")
    } else {
        format!("{PROJECTS_DIR}/{name}")
    }
}

/// ルートからのパス（ファイル）の置き場所を決める。
pub fn classify(rel: &str) -> Info {
    let mut info = Info {
        path: rel.to_string(),
        ..Info::default()
    };
    let parts: Vec<&str> = rel.split('/').collect();
    match parts[0] {
        ARCHIVE_DIR => {
            if parts.len() < 4 || parts[1] != PROJECTS_DIR {
                info.set_problem("ルートの archive/ には projects/<プロジェクト>/ だけを置く");
                return info;
            }
            info.project_archived = true;
            info.archived = true;
            classify_in_project(&mut info, parts[2], &parts[3..]);
            return info;
        }
        PROJECTS_DIR => {
            if parts.len() < 3 {
                info.set_problem("projects/ の中にはプロジェクトのディレクトリだけを置く");
                return info;
            }
            classify_in_project(&mut info, parts[1], &parts[2..]);
            return info;
        }
        _ => {}
    }
    if parts.len() == 1 {
        if parts[0] == "README.md" {
            info.kind = Some(Kind::Readme);
            return info;
        }
        info.set_problem(
            "ルートに置けるファイルは README.md だけ（ほかは予約名のディレクトリかプロジェクトに）",
        );
        return info;
    }
    match parts[0] {
        "assets" => {
            info.kind = Some(Kind::Asset);
            info.asset = true;
            if parts.len() != 2 {
                info.set_problem("assets/ の中はフラットにする");
            }
            return info;
        }
        "principles" | "knowledge" => {
            let r = rule_by_dir(parts[0]).expect("principles と knowledge の形はある");
            classify_kind_file(&mut info, r, &parts);
            return info;
        }
        _ => {}
    }
    info.set_problem("ルートに置けるのは予約名だけ（プロジェクトは projects/ の下に置く）");
    info
}

fn classify_in_project(info: &mut Info, project: &str, rest: &[&str]) {
    info.project = project.to_string();
    if rest.len() == 1 {
        if rest[0] == "README.md" {
            info.kind = Some(Kind::Readme);
            return;
        }
        info.set_problem("プロジェクトの直下に置けるファイルは README.md だけ");
        return;
    }
    match rest[0] {
        "assets" => {
            info.kind = Some(Kind::Asset);
            info.asset = true;
            if rest.len() != 2 {
                info.set_problem("assets/ の中はフラットにする");
            }
            return;
        }
        ARCHIVE_DIR => {
            if info.archived && !info.project_archived {
                info.set_problem("archive/ の中に archive/ を置かない");
                return;
            }
            info.archived = true;
            if rest.len() < 3 {
                info.set_problem(
                    "プロジェクトの archive/ の中はプロジェクトと同じ形（archive/tasks/・archive/artifacts/）",
                );
                return;
            }
            match rule_by_dir(rest[1]) {
                Some(r) if r.archivable => classify_kind_file(info, r, &rest[1..]),
                _ => {
                    info.set_problem("プロジェクトの archive/ に置くのは tasks/ と artifacts/ だけ")
                }
            }
            return;
        }
        _ => {}
    }
    match rule_by_dir(rest[0]) {
        Some(r) if r.in_project => classify_kind_file(info, r, rest),
        _ => info.set_problem(format!(
            "プロジェクトに置けないディレクトリ（{}/）",
            rest[0]
        )),
    }
}

/// 種類のディレクトリ（sub[0]）の中のファイルを決める。
fn classify_kind_file(info: &mut Info, r: &KindRule, sub: &[&str]) {
    info.kind = Some(r.kind);
    let name = sub[sub.len() - 1];
    if sub.len() >= 3 && sub[sub.len() - 2] == "assets" && r.assets {
        info.kind = Some(Kind::Asset);
        info.asset = true;
        if !r.nested && sub.len() != 3 {
            info.set_problem("assets/ の中はフラットにする");
        }
        return;
    }
    if sub[1..sub.len() - 1].contains(&"assets") {
        info.set_problem("assets/ の中はフラットにする");
        return;
    }
    if !r.nested && sub.len() != 2 {
        info.set_problem(format!("{}/ の中はフラットにする", r.dir));
        return;
    }
    if !name.ends_with(".md") {
        if r.assets {
            info.set_problem(format!("静的ファイルは {}/assets/ に置く", r.dir));
        } else {
            info.set_problem(format!("{}/ には .md だけを置く", r.dir));
        }
    }
}

/// ファイル名の形の問題を返す（無ければ None）。
pub fn name_problem(info: &Info) -> Option<String> {
    let r = info.kind.and_then(rule_of)?;
    if !r.dated || info.asset {
        return None;
    }
    let Some((date, _)) = split_dated(info.name()) else {
        return Some("名前に日付が無い（YYYY-MM-DD-<slug>.md にする）".to_string());
    };
    if !is_date(date) {
        return Some(format!("名前の日付が日付になっていない（{date}）"));
    }
    None
}

/// YYYY-MM-DD が暦の上にある日か。
pub fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| -> Option<u32> {
        let p = &s[r];
        p.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| p.parse().ok())?
    };
    let (Some(y), Some(m), Some(d)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&d)
}

/// 名前が一意であるべき範囲の鍵（プロジェクトの archive/ の中と外は同じ範囲）。
pub fn unique_scope(info: &Info) -> Option<String> {
    let r = info.kind.and_then(rule_of)?;
    Some(format!("{}|{}", info.project_dir(), r.dir))
}

/// YYYY-MM-DD-<slug>.md を日付と slug に分ける。
pub fn split_dated(name: &str) -> Option<(&str, &str)> {
    let c = DATED_NAME.captures(name)?;
    Some((c.get(1)?.as_str(), c.get(2)?.as_str()))
}

/// プロジェクトの中のファイルを archive/ に移した先のパスを返す。
pub fn archive_path(info: &Info) -> String {
    if info.project_archived || info.project.is_empty() {
        return info.path.clone();
    }
    let pd = info.project_dir();
    let rest = info
        .path
        .strip_prefix(&format!("{pd}/"))
        .unwrap_or(&info.path);
    format!("{pd}/{ARCHIVE_DIR}/{rest}")
}

/// archive/ の中のファイルの元の場所を返す。
pub fn unarchive_path(info: &Info) -> String {
    let pd = info.project_dir();
    if pd.is_empty() {
        return info.path.clone();
    }
    let rest = info
        .path
        .strip_prefix(&format!("{pd}/"))
        .unwrap_or(&info.path);
    match rest.strip_prefix(&format!("{ARCHIVE_DIR}/")) {
        Some(r) => format!("{pd}/{r}"),
        None => info.path.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert!(is_date("2024-02-29"));
        assert!(!is_date("2026-02-29"));
        assert!(!is_date("2026-13-01"));
        assert!(!is_date("2026-00-10"));
        assert!(!is_date("2026-04-31"));
    }
}
