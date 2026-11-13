//! リンクと、その先の解決。

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::path::{dir, join};

/// リンクの形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkForm {
    /// 相対パス
    #[default]
    Relative,
    /// / 始まり、file:
    Absolute,
    /// ~ 始まり
    Home,
    /// [[名前]]
    Wiki,
    /// URL
    External,
    /// #だけ
    Anchor,
}

/// ファイルの中のリンク 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Link {
    /// "body" か frontmatter のキー
    pub key: String,
    /// 本文のリンクの文字
    pub text: String,
    /// 書いてあるとおりの先（アンカー込み）
    pub raw: String,
    /// ファイルの行（0 始まり）
    pub line: usize,
    /// 行の中のバイト位置（None なら書き換えられない）
    pub start: Option<usize>,
    pub form: LinkForm,
    /// Relative: ルートからのパス（外なら ../ 始まり）。Absolute・Home: 絶対パス。Wiki: 名前
    pub target: String,
    pub anchor: String,
}

static SCHEME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9+.-]*:").unwrap());

/// リンクの形を決め、相対パスならルートからのパスにする。from はリンクを書いたファイルのルートからのパス。
pub fn resolve(from: &str, l: &mut Link) {
    let raw = l.raw.as_str();
    let (p, anchor) = raw.split_once('#').unwrap_or((raw, ""));
    l.anchor = anchor.to_string();
    if raw.starts_with('#') {
        l.form = LinkForm::Anchor;
    } else if raw.starts_with("file:") {
        l.form = LinkForm::Absolute;
        l.target = file_url_path(p).unwrap_or_default();
    } else if SCHEME_RE.is_match(raw) {
        l.form = LinkForm::External;
    } else if raw.starts_with('/') {
        l.form = LinkForm::Absolute;
        l.target = unescape(p);
    } else if raw.starts_with('~') {
        l.form = LinkForm::Home;
        l.target = unescape(p);
    } else {
        l.form = LinkForm::Relative;
        l.target = join(&[&dir(from), &unescape(p)]);
    }
}

/// `file:` の URL のパス（Go の url.Parse の Path）。読めなければ None。
fn file_url_path(u: &str) -> Option<String> {
    let rest = &u["file:".len()..];
    let rest = rest.split_once('?').map_or(rest, |(p, _)| p);
    let path = match rest.strip_prefix("//") {
        Some(r) => r.find('/').map_or("", |i| &r[i..]),
        None if rest.starts_with('/') => rest,
        // file:a.md のような不透明な形にはパスが無い
        None => "",
    };
    percent_decode(path)
}

/// %XX を戻す（Go の url.PathUnescape）。戻せなければそのまま。
pub fn unescape(p: &str) -> String {
    percent_decode(p).unwrap_or_else(|| p.to_string())
}

fn percent_decode(p: &str) -> Option<String> {
    if !p.contains('%') {
        return Some(p.to_string());
    }
    let b = p.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = |c: u8| (c as char).to_digit(16);
            let hi = hex(*b.get(i + 1)?)?;
            let lo = hex(*b.get(i + 2)?)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    // Go は UTF-8 でないバイト列もそのまま持つが、ここでは戻さずに元の文字列を使う
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(from: &str, raw: &str) -> Link {
        let mut l = Link {
            raw: raw.to_string(),
            ..Link::default()
        };
        resolve(from, &mut l);
        l
    }

    #[test]
    fn forms() {
        let from = "projects/a/tasks/x.md";
        let l = link(from, "../artifacts/b%20c.md#節");
        assert_eq!(
            (l.form, l.target.as_str(), l.anchor.as_str()),
            (LinkForm::Relative, "projects/a/artifacts/b c.md", "節")
        );
        assert_eq!(link(from, "#節").form, LinkForm::Anchor);
        assert_eq!(link(from, "https://example.com").form, LinkForm::External);
        assert_eq!(link(from, "mailto:a@example.com").form, LinkForm::External);
        let l = link(from, "file:///Users/a/b%20c.md");
        assert_eq!(
            (l.form, l.target.as_str()),
            (LinkForm::Absolute, "/Users/a/b c.md")
        );
        assert_eq!(link(from, "file:a.md").target, "");
        assert_eq!(link(from, "/abs/x.md").target, "/abs/x.md");
        assert_eq!(link(from, "~/x.md").form, LinkForm::Home);
        assert_eq!(link(from, "../../../../x.md").target, "../x.md");
        assert_eq!(link(from, "bad%zz.md").target, "projects/a/tasks/bad%zz.md");
        assert_eq!(
            link("README.md", "projects/a/README.md").target,
            "projects/a/README.md"
        );
    }
}
