//! リンクと、その先の解決。

use std::sync::LazyLock;

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use regex::Regex;
use relative_path::RelativePath;
use serde::Serialize;
use url::Url;

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
    /// 先（本文のものは CommonMark か HTML で読んだもの。アンカー込み）
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
        l.target = file_url_path(raw).unwrap_or_default();
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
        l.target = target(from, p);
    }
}

/// from（ルートからのパス）に書いた相対パスのリンクの先を、ルートからのパスにする（外なら ../ 始まり）。
pub fn target(from: &str, rel: &str) -> String {
    dir_of(from).join_normalized(unescape(rel)).into_string()
}

/// ファイル（ルートからのパス）のあるディレクトリ。ルートのファイルなら空。
fn dir_of(path: &str) -> &RelativePath {
    let p = RelativePath::new(path);
    p.parent().unwrap_or(p)
}

/// from（ルートからのパス）に書く、to（ルートからのパス）へのリンクの文字。
pub fn link_text(from: &str, to: &str, anchor: &str) -> String {
    let rel = dir_of(from).relative(to);
    let s = utf8_percent_encode(rel.as_str(), LINK_ESCAPE).to_string();
    if anchor.is_empty() {
        s
    } else {
        format!("{s}#{anchor}")
    }
}

/// リンクに書くときにエスケープする文字。空白はリンクを切り、% と # は読むときに別の意味になる。
const LINK_ESCAPE: &AsciiSet = &CONTROLS.add(b' ').add(b'%').add(b'#');

/// `file:` の URL のパス。読めなければ None。
fn file_url_path(u: &str) -> Option<String> {
    let path = Url::parse(u).ok()?.to_file_path().ok()?;
    path.to_str().map(str::to_string)
}

/// %XX を戻す。UTF-8 にならなければそのまま。
pub fn unescape(p: &str) -> String {
    percent_decode_str(p)
        .decode_utf8()
        .map_or_else(|_| p.to_string(), |s| s.into_owned())
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
        // file: の URL のパスは url に任せる（ホストの無い file:a.md も /a.md と読む）
        assert_eq!(link(from, "file:a.md").target, "/a.md");
        assert_eq!(link(from, "/abs/x.md").target, "/abs/x.md");
        assert_eq!(link(from, "~/x.md").form, LinkForm::Home);
        assert_eq!(link(from, "../../../../x.md").target, "../x.md");
        assert_eq!(link(from, "bad%zz.md").target, "projects/a/tasks/bad%zz.md");
        assert_eq!(
            link(from, "b%20c%zz.md").target,
            "projects/a/tasks/b c%zz.md"
        );
        assert_eq!(
            link("README.md", "projects/a/README.md").target,
            "projects/a/README.md"
        );
    }

    /// 書くリンクは、読むと元の先に戻る。
    #[test]
    fn link_text_round_trips() {
        let from = "projects/a/tasks/x.md";
        for (to, anchor, want) in [
            (
                "projects/a/artifacts/b c.md",
                "節",
                "../artifacts/b%20c.md#節",
            ),
            ("projects/a/tasks/y.md", "", "y.md"),
            (
                "projects/a/artifacts/assets/50%#1.png",
                "",
                "../artifacts/assets/50%25%231.png",
            ),
            ("README.md", "", "../../../README.md"),
        ] {
            let text = link_text(from, to, anchor);
            assert_eq!(text, want);
            let l = link(from, &text);
            assert_eq!((l.target.as_str(), l.anchor.as_str()), (to, anchor));
        }
        assert_eq!(
            link_text("README.md", "projects/a/README.md", ""),
            "projects/a/README.md"
        );
    }
}
