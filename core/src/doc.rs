//! ツリーの中のファイル 1 つ。.md なら中身（frontmatter・見出し・リンク）を読んだもの。

use std::sync::LazyLock;

use regex::Regex;
use serde::{Serialize, Serializer};

use crate::frontmatter::{self, Field, Frontmatter, Scalar, Value};
use crate::link::{Link, LinkForm, resolve};
use crate::path::Info;
use crate::rules::{Kind, NOW_SECTION, link_keys};

/// 見出し 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    /// ファイルの行（0 始まり）
    pub line: usize,
    /// 節の終わり（次の同じか浅い見出しの行。無ければファイルの終わり。末尾の空行は含めない）
    pub end: usize,
}

/// ツリーの中のファイル 1 つ。
#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub info: Info,
    pub is_md: bool,
    /// 行（改行を除く）
    pub lines: Vec<String>,
    /// `---` で始まり、閉じる行がある
    pub has_fm: bool,
    /// frontmatter が読めないときの説明
    pub fm_error: Option<String>,
    /// frontmatter を閉じる `---` の行（has_fm のとき）
    pub fm_end: usize,
    /// 読めた frontmatter（無いか読めなければ None）
    pub frontmatter: Option<Frontmatter>,
    pub body_start: usize,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
}

/// Go の正規表現の `\s`（ASCII の空白）。Rust の `\s` は Unicode の空白も含むので、使わない。
macro_rules! ws {
    () => {
        r"[\t\n\f\r ]"
    };
}

static HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(#{1,6})",
        ws!(),
        r"+(.*?)",
        ws!(),
        r"*#*",
        ws!(),
        r"*$"
    ))
    .unwrap()
});
static MD_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(!?)\[((?:[^\[\]]|\[[^\]]*\])*)\]\(",
        ws!(),
        r"*(<[^>]*>|[^)\t\n\f\r ]+)(?:",
        ws!(),
        r#"+(?:"[^"]*"|'[^']*'|\([^)]*\)))?"#,
        ws!(),
        r"*\)"
    ))
    .unwrap()
});
static REF_DEF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^",
        ws!(),
        r"{0,3}\[[^\]]+\]:",
        ws!(),
        r"*(<[^>]*>|[^\t\n\f\r ]+)"
    ))
    .unwrap()
});
static WIKI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").unwrap());
static INLINE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`+[^`]*`+").unwrap());
static LIST_MARK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^",
        ws!(),
        r"*(?:[-*+]",
        ws!(),
        r"+(?:\[[ xX]\]",
        ws!(),
        r"+)?|[0-9]+\.",
        ws!(),
        r"+)"
    ))
    .unwrap()
});

impl Doc {
    /// .md でないファイル（静的ファイルなど）。
    pub fn file(info: Info) -> Doc {
        Doc {
            info,
            ..Doc::default()
        }
    }

    /// .md の中身を読む。
    pub fn parse(info: Info, content: &str) -> Doc {
        let content = content.replace("\r\n", "\n");
        let mut d = Doc {
            info,
            is_md: true,
            lines: content.split('\n').map(str::to_string).collect(),
            ..Doc::default()
        };
        d.parse_frontmatter();
        d.parse_body();
        d
    }

    fn parse_frontmatter(&mut self) {
        if self.lines.first().map(|l| l.trim()) != Some("---") {
            return;
        }
        let Some(end) = self.lines[1..]
            .iter()
            .position(|l| {
                let t = l.trim_end_matches([' ', '\t']);
                t == "---" || t == "..."
            })
            .map(|i| i + 1)
        else {
            self.fm_error = Some("frontmatter が閉じていない".to_string());
            return;
        };
        self.has_fm = true;
        self.fm_end = end;
        self.body_start = end + 1;
        // 閉じる --- の前の改行も YAML のうち（最後の | や > の値は改行で終わる）
        let text: String = self.lines[1..end]
            .iter()
            .map(|l| format!("{l}\n"))
            .collect();
        let fm = match frontmatter::parse(&text) {
            Ok(fm) => fm,
            Err(e) => {
                self.fm_error = Some(e.to_string());
                return;
            }
        };
        let keys = link_keys(self.info.kind);
        for f in &fm.fields {
            if !keys.contains(&f.key.as_str()) {
                continue;
            }
            for v in &f.scalars {
                if v.value.trim().is_empty() {
                    continue;
                }
                let start = self.locate(v);
                self.links.push(Link {
                    key: f.key.clone(),
                    raw: v.value.clone(),
                    line: v.line,
                    start,
                    ..Link::default()
                });
            }
        }
        self.frontmatter = Some(fm);
    }

    /// frontmatter の値が行の何バイト目から始まるかを探す。
    pub(crate) fn locate(&self, v: &Scalar) -> Option<usize> {
        let s = self.lines.get(v.line)?;
        let mut off = s.char_indices().nth(v.column).map_or(s.len(), |(i, _)| i);
        if v.quoted {
            off += 1;
        }
        if s.get(off..).is_some_and(|rest| rest.starts_with(&v.value)) {
            return Some(off);
        }
        s.find(&v.value)
    }

    fn parse_body(&mut self) {
        let mut fence = "";
        let mut body_links = Vec::new();
        for i in self.body_start..self.lines.len() {
            let line = &self.lines[i];
            let trim = line.trim_start_matches(' ');
            if !fence.is_empty() {
                if trim.starts_with(fence) {
                    fence = "";
                }
                continue;
            }
            if trim.starts_with("```") {
                fence = "```";
                continue;
            }
            if trim.starts_with("~~~") {
                fence = "~~~";
                continue;
            }
            if let Some(c) = HEADING_RE.captures(line) {
                self.headings.push(Heading {
                    level: c[1].len(),
                    text: c[2].to_string(),
                    line: i,
                    end: 0,
                });
            }
            let masked = INLINE_RE.replace_all(line, |c: &regex::Captures| " ".repeat(c[0].len()));
            for c in MD_LINK_RE.captures_iter(&masked) {
                let (Some(text), Some(url)) = (c.get(2), c.get(3)) else {
                    continue;
                };
                let (Some(text), Some((raw, start))) =
                    (line.get(text.range()), angle(line, url.range()))
                else {
                    continue;
                };
                body_links.push(Link {
                    key: "body".to_string(),
                    text: text.to_string(),
                    raw,
                    line: i,
                    start: Some(start),
                    ..Link::default()
                });
            }
            if let Some(url) = REF_DEF_RE.captures(&masked).and_then(|c| c.get(1))
                && let Some((raw, start)) = angle(line, url.range())
            {
                body_links.push(Link {
                    key: "body".to_string(),
                    raw,
                    line: i,
                    start: Some(start),
                    ..Link::default()
                });
            }
            for c in WIKI_RE.captures_iter(&masked) {
                let (whole, name) = (c.get(0).unwrap(), c.get(1).unwrap());
                let (Some(raw), Some(target)) = (line.get(whole.range()), line.get(name.range()))
                else {
                    continue;
                };
                body_links.push(Link {
                    key: "body".to_string(),
                    raw: raw.to_string(),
                    line: i,
                    start: Some(whole.start()),
                    form: LinkForm::Wiki,
                    target: target.to_string(),
                    ..Link::default()
                });
            }
        }
        self.links.extend(body_links);
        let from = self.info.path.clone();
        for l in &mut self.links {
            if l.form != LinkForm::Wiki {
                resolve(&from, l);
            }
        }
        for i in 0..self.headings.len() {
            self.headings[i].end = self.section_end(i);
        }
    }

    /// i 番目の見出しの節の終わり。
    fn section_end(&self, i: usize) -> usize {
        let h = &self.headings[i];
        let mut end = self.headings[i + 1..]
            .iter()
            .find(|h2| h2.level <= h.level)
            .map_or(self.lines.len(), |h2| h2.line);
        while end > h.line + 1 && self.lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        end
    }

    // frontmatter の読み出し

    fn field(&self, key: &str) -> Option<&Field> {
        self.frontmatter.as_ref()?.get(key)
    }

    /// frontmatter にキーがあるか。
    pub fn has_key(&self, key: &str) -> bool {
        self.field(key).is_some()
    }

    /// frontmatter の値を文字列で返す（配列ならカンマでつなぐ）。
    pub fn str(&self, key: &str) -> String {
        match self.field(key).map(|f| &f.value) {
            Some(Value::Str(s)) => s.clone(),
            Some(Value::List(l)) => l.join(", "),
            Some(Value::Null) | None => String::new(),
        }
    }

    /// frontmatter の値を配列で返す（1 つの値なら 1 件の配列）。
    pub fn list(&self, key: &str) -> Vec<String> {
        self.field(key)
            .map(|f| f.scalars.iter().map(|s| s.value.clone()).collect())
            .unwrap_or_default()
    }

    /// frontmatter のキーを順に返す。
    pub fn fm_keys(&self) -> Vec<&str> {
        self.frontmatter
            .as_ref()
            .map(|fm| fm.keys().collect())
            .unwrap_or_default()
    }

    // 本文の読み出し

    /// 見出し（最初の # の見出し。無ければ最初の見出し）。
    pub fn title(&self) -> &str {
        self.headings
            .iter()
            .find(|h| h.level == 1)
            .or(self.headings.first())
            .map_or("", |h| h.text.as_str())
    }

    /// description を 1 行にしたもの。
    pub fn description(&self) -> String {
        one_line(&self.str("description"))
    }

    /// 「いま」の 1 行目。
    pub fn now(&self) -> String {
        let Some((start, end)) = self.section_range(NOW_SECTION) else {
            return String::new();
        };
        self.lines[start + 1..end]
            .iter()
            .map(|l| l.trim())
            .find(|t| !t.is_empty())
            .map(|t| LIST_MARK.replace(t, "").trim().to_string())
            .unwrap_or_default()
    }

    /// status（無ければ空）。
    pub fn status(&self) -> String {
        self.str("status")
    }

    /// needs_human が true か。
    pub fn needs_human(&self) -> bool {
        self.str("needs_human") == "true"
    }

    /// 見出しの行と、節の終わり（次の同じか浅い見出しの行。無ければファイルの終わり）。
    pub fn section_range(&self, name: &str) -> Option<(usize, usize)> {
        self.headings
            .iter()
            .find(|h| h.text == name)
            .map(|h| (h.line, h.end))
    }

    /// ## の見出しを順に返す。
    pub fn h2(&self) -> impl Iterator<Item = &Heading> {
        self.headings.iter().filter(|h| h.level == 2)
    }

    /// ファイルの中身。
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }
}

/// `<…>` で囲んだ先なら外し、書いてあるとおりの先と行の中の位置を返す。
fn angle(line: &str, r: std::ops::Range<usize>) -> Option<(String, usize)> {
    let raw = line.get(r.clone())?;
    match raw.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        Some(inner) => Some((inner.to_string(), r.start + 1)),
        None => Some((raw.to_string(), r.start)),
    }
}

/// 空白を 1 つにつめて 1 行にする。
pub fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// アプリに渡す形。行は持たず、本文から読み取ったものを出す。
impl Serialize for Doc {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Out<'a> {
            path: &'a str,
            kind: Option<Kind>,
            project: &'a str,
            archived: bool,
            project_archived: bool,
            asset: bool,
            problem: Option<&'a str>,
            is_md: bool,
            frontmatter: Option<&'a Frontmatter>,
            frontmatter_error: Option<&'a str>,
            title: &'a str,
            description: String,
            now: String,
            status: String,
            needs_human: bool,
            headings: &'a [Heading],
            links: &'a [Link],
        }
        Out {
            path: &self.info.path,
            kind: self.info.kind,
            project: &self.info.project,
            archived: self.info.archived,
            project_archived: self.info.project_archived,
            asset: self.info.asset,
            problem: self.info.problem.as_deref(),
            is_md: self.is_md,
            frontmatter: self.frontmatter.as_ref(),
            frontmatter_error: self.fm_error.as_deref(),
            title: self.title(),
            description: self.description(),
            now: self.now(),
            status: self.status(),
            needs_human: self.needs_human(),
            headings: &self.headings,
            links: &self.links,
        }
        .serialize(s)
    }
}
