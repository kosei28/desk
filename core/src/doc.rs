//! ツリーの中のファイル 1 つ。.md なら中身（frontmatter・見出し・リンク）を読んだもの。

use std::ops::Range;
use std::sync::LazyLock;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde::{Serialize, Serializer};

use crate::frontmatter::{self, Field, Frontmatter, Scalar, Value};
use crate::html;
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

/// 「いま」の行の頭のリストの印（`- `・`* `・`+ `・`1. `、チェックボックス）。
static LIST_MARK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[-*+][ \t]+(?:\[[ xX]\][ \t]+)?|[0-9]+\.[ \t]+)").unwrap());

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

    /// 本文を CommonMark として読み、見出しとリンク（生の HTML の中のものも）を拾う。
    fn parse_body(&mut self) {
        let body = self.lines[self.body_start..].join("\n");
        let starts: Vec<usize> = std::iter::once(0)
            .chain(body.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        // 本文の中の位置 → (ファイルの行, 行の中の位置)
        let pos = |off: usize| {
            let i = starts.partition_point(|&s| s <= off) - 1;
            (self.body_start + i, off - starts[i])
        };
        let parser = Parser::new_ext(&body, Options::ENABLE_WIKILINKS);
        // 本文のリンクと、本文の中の位置（書いてある順に並べるため）
        let mut body_links: Vec<(usize, Link)> = Vec::new();
        // 参照の定義（書いてある順に）。同じ名前の定義は CommonMark どおり最初のものだけ
        let mut defs: Vec<(Range<usize>, String)> = parser
            .reference_definitions()
            .iter()
            .map(|(_, d)| (d.span.clone(), d.dest.to_string()))
            .collect();
        defs.sort_by_key(|(span, _)| span.start);
        for (span, dest) in defs {
            let Some(at) = dest_in(&body, span, "]:", &dest) else {
                continue;
            };
            let (line, start) = pos(at);
            let l = Link {
                key: "body".to_string(),
                raw: dest,
                line,
                start: Some(start),
                ..Link::default()
            };
            body_links.push((at, l));
        }
        let mut headings = Vec::new();
        // 読んでいる見出し（段、行、中身の範囲）
        let mut heading: Option<(usize, usize, Option<Range<usize>>)> = None;
        // 開いているリンクと、その文字の範囲（画像はリンクの中に入れ子になりうる）
        let mut open: Vec<(usize, Link, Option<Range<usize>>)> = Vec::new();
        // 入っている引用・リスト・脚注の数。見出しはこの外のものだけを文書の節にする
        let mut nested = 0usize;
        // 読んでいるブロックの HTML の行の範囲
        let mut html_block: Vec<Range<usize>> = Vec::new();
        for (ev, r) in parser.into_offset_iter() {
            match &ev {
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, line, inner)) = heading.take() {
                        headings.push(Heading {
                            level,
                            text: source_text(&body, inner),
                            line,
                            end: 0,
                        });
                    }
                    continue;
                }
                Event::End(TagEnd::Link | TagEnd::Image) => {
                    if let Some((at, mut l, inner)) = open.pop()
                        && !l.raw.is_empty()
                    {
                        l.text = source_text(&body, inner);
                        body_links.push((at, l));
                    }
                    continue;
                }
                _ => {}
            }
            // 見出しとリンクの中身は、中のイベントの範囲を合わせたもの
            if let Some((_, _, inner)) = &mut heading {
                widen(inner, &r);
            }
            for (_, _, inner) in &mut open {
                widen(inner, &r);
            }
            match ev {
                Event::Start(Tag::Heading { level, .. }) if nested == 0 => {
                    heading = Some((level as usize, pos(r.start).0, None));
                }
                Event::Start(Tag::BlockQuote(_) | Tag::List(_) | Tag::FootnoteDefinition(_)) => {
                    nested += 1
                }
                Event::End(
                    TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::FootnoteDefinition,
                ) => nested -= 1,
                Event::Start(
                    Tag::Link {
                        link_type,
                        dest_url,
                        ..
                    }
                    | Tag::Image {
                        link_type,
                        dest_url,
                        ..
                    },
                ) => {
                    let at = r.start;
                    open.push((at, inline_link(&body, r, link_type, &dest_url, &pos), None));
                }
                Event::InlineHtml(_) => body_links.extend(html_links(&body, &[r], &pos)),
                Event::Html(_) => html_block.push(r),
                Event::End(TagEnd::HtmlBlock) => {
                    body_links.extend(html_links(&body, &html_block, &pos));
                    html_block.clear();
                }
                _ => {}
            }
        }
        body_links.sort_by_key(|(at, _)| *at);
        self.headings = headings;
        self.links.extend(body_links.into_iter().map(|(_, l)| l));
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

/// 範囲を r まで広げる。
fn widen(inner: &mut Option<Range<usize>>, r: &Range<usize>) {
    *inner = Some(match inner.take() {
        Some(i) => i.start.min(r.start)..i.end.max(r.end),
        None => r.clone(),
    });
}

/// 書いてあるとおりの文字（複数行なら行を空白でつなぐ）。
fn source_text(body: &str, r: Option<Range<usize>>) -> String {
    let s = r.and_then(|r| body.get(r)).unwrap_or_default();
    s.lines().map(str::trim).collect::<Vec<_>>().join(" ")
}

/// 本文のリンク 1 つ（範囲 r はリンク全体）。拾わないもの（参照の形の使う側、メールの autolink）は raw が空。
fn inline_link(
    body: &str,
    r: Range<usize>,
    link_type: LinkType,
    dest: &str,
    pos: &impl Fn(usize) -> (usize, usize),
) -> Link {
    let mut l = Link {
        key: "body".to_string(),
        ..Link::default()
    };
    match link_type {
        LinkType::Inline => {
            l.raw = dest.to_string();
            // 書いてあるとおりの先（エスケープなどが無い）なら、書き換えられるように位置を持つ
            let at = dest_in(body, r.clone(), "](", dest);
            let (line, start) = pos(at.unwrap_or(r.start));
            l.line = line;
            l.start = at.map(|_| start);
        }
        LinkType::Autolink => {
            l.raw = dest.to_string();
            (l.line, _) = pos(r.start);
            l.start = Some(pos(r.start + 1).1);
        }
        LinkType::WikiLink { .. } => {
            l.raw = body[r.clone()].to_string();
            l.form = LinkForm::Wiki;
            l.target = dest.to_string();
            let (line, start) = pos(r.start);
            (l.line, l.start) = (line, Some(start));
        }
        // 参照の形は定義の側で拾う（1 つの定義を何度使っても 1 本）
        _ => {}
    }
    l
}

/// 生の HTML のリンク。HTML は本文の中の範囲 parts をつないだもの（ブロックの HTML は行ごとに、引用やリストの印を除いて渡る）。
fn html_links(
    body: &str,
    parts: &[Range<usize>],
    pos: &impl Fn(usize) -> (usize, usize),
) -> Vec<(usize, Link)> {
    let html: String = parts.iter().map(|r| &body[r.clone()]).collect();
    // HTML の中の位置 → 本文の中の位置
    let in_body = |mut off: usize| {
        for r in parts {
            if off < r.len() {
                return r.start + off;
            }
            off -= r.len();
        }
        parts.last().map_or(0, |r| r.end)
    };
    html::links(&html)
        .into_iter()
        .map(|h| {
            // 書いてあるとおりの先（行をまたいで引用の印などを挟まない）なら、書き換えられるように位置を持つ
            let at =
                h.at.map(in_body)
                    .filter(|&at| body[at..].starts_with(&h.raw));
            let at_or_attr = at.unwrap_or_else(|| in_body(h.attr));
            let (line, start) = pos(at_or_attr);
            let l = Link {
                key: "body".to_string(),
                raw: h.raw,
                line,
                start: at.map(|_| start),
                ..Link::default()
            };
            (at_or_attr, l)
        })
        .collect()
}

/// 範囲 r の中で、印（`](` か `]:`）の後に書いてある先 dest の位置。書いてあるとおりでなければ None。
fn dest_in(body: &str, r: Range<usize>, mark: &str, dest: &str) -> Option<usize> {
    let s = body.get(r.clone())?;
    s.rmatch_indices(mark).find_map(|(i, _)| {
        let after = &s[i + mark.len()..];
        let rest = after.trim_start_matches([' ', '\t', '\n']);
        let rest = rest.strip_prefix('<').unwrap_or(rest);
        rest.starts_with(dest)
            .then(|| r.start + s.len() - rest.len())
    })
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
