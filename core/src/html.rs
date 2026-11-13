//! 本文の生の HTML の中のリンク。

use html5gum::{DefaultEmitter, Token, Tokenizer};

/// リンクとして読む要素と属性（リンクと画像の先）。
const LINK_ATTRS: [(&str, &str); 3] = [("a", "href"), ("img", "src"), ("source", "srcset")];

/// HTML の中のリンク 1 つ。
pub struct HtmlLink {
    /// 先（文字参照を戻し、前後の空白を除いたもの）
    pub raw: String,
    /// 属性の位置
    pub attr: usize,
    /// 先が書いてあるとおり（文字参照などが無い）なら、その位置
    pub at: Option<usize>,
}

/// HTML の中のリンクを、書いてある順に返す。コメントと script・style などの中身は読まない。
pub fn links(html: &str) -> Vec<HtmlLink> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut out = Vec::new();
    for token in Tokenizer::new_with_emitter(html, emitter) {
        let Ok(Token::StartTag(tag)) = token else {
            continue;
        };
        for (element, name) in LINK_ATTRS {
            if *tag.name != element.as_bytes() {
                continue;
            }
            let Some(v) = tag.attributes.get(name.as_bytes()) else {
                continue;
            };
            let value = String::from_utf8_lossy(&v.value);
            let attr = v.span.start;
            let at = value_start(html, attr, name).filter(|&at| html[at..].starts_with(&*value));
            let urls = if name == "srcset" {
                srcset_urls(&value)
            } else {
                let url = value.trim_matches(is_space);
                let off = value.len() - value.trim_start_matches(is_space).len();
                if url.is_empty() {
                    vec![]
                } else {
                    vec![(off, url)]
                }
            };
            out.extend(urls.into_iter().map(|(off, url)| HtmlLink {
                raw: url.to_string(),
                attr,
                at: at.map(|at| at + off),
            }));
        }
    }
    out
}

/// 位置 attr から書いてある属性 name の値が始まる位置（引用符の内側）。値が無ければ None。
fn value_start(html: &str, attr: usize, name: &str) -> Option<usize> {
    let rest = html.get(attr + name.len()..)?;
    let rest = rest.trim_start_matches(is_space).strip_prefix('=')?;
    let rest = rest.trim_start_matches(is_space);
    let rest = rest.strip_prefix(['"', '\'']).unwrap_or(rest);
    Some(html.len() - rest.len())
}

/// srcset の候補の URL と、値の中の位置。HTML の仕様の「srcset 属性を読む」のうち、URL を取り出すところ。
fn srcset_urls(v: &str) -> Vec<(usize, &str)> {
    let b = v.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        while i < b.len() && (is_space(b[i] as char) || b[i] == b',') {
            i += 1;
        }
        if i == b.len() {
            return out;
        }
        let start = i;
        while i < b.len() && !is_space(b[i] as char) {
            i += 1;
        }
        let url = v[start..i].trim_end_matches(',');
        out.push((start, url));
        if url.len() < i - start {
            continue;
        }
        // 記述子（`2x`・`100w` など）を、括弧の外のカンマまで飛ばす
        let mut in_parens = false;
        while i < b.len() {
            match b[i] {
                b'(' => in_parens = true,
                b')' => in_parens = false,
                b',' if !in_parens => break,
                _ => {}
            }
            i += 1;
        }
    }
}

/// HTML の空白（ASCII whitespace）。
fn is_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn got(html: &str) -> Vec<(String, Option<usize>)> {
        links(html).into_iter().map(|l| (l.raw, l.at)).collect()
    }

    #[test]
    fn link_attrs() {
        let html = r#"<a href="a.md" title="t">a</a><IMG SRC=b.png alt=x><img src = ' c.png '><link href="d.css"><a name="e">"#;
        assert_eq!(
            got(html),
            [
                ("a.md".to_string(), Some(9)),
                ("b.png".to_string(), Some(39)),
                ("c.png".to_string(), Some(64)),
            ]
        );
    }

    #[test]
    fn char_refs_are_not_as_written() {
        let l = &links(r#"<a href="x&amp;y.md">"#)[0];
        assert_eq!((l.raw.as_str(), l.attr, l.at), ("x&y.md", 3, None));
    }

    #[test]
    fn comments_and_raw_text_are_not_read() {
        let html = r#"<!-- <a href="a.md"> --><script><a href="b.md"></script><style><img src="c.png"></style><a href="d.md">"#;
        assert_eq!(got(html), [("d.md".to_string(), Some(97))]);
    }

    #[test]
    fn srcset() {
        assert_eq!(
            srcset_urls(" a.png 1x, b,c.png 2x,d.png,, e.png (1, 2) 100w,f.png"),
            [
                (1, "a.png"),
                (11, "b,c.png"),
                (22, "d.png"),
                (30, "e.png"),
                (48, "f.png")
            ]
        );
        let html = r#"<source srcset="a.png 1x, b.png 2x">"#;
        assert_eq!(
            got(html),
            [
                ("a.png".to_string(), Some(16)),
                ("b.png".to_string(), Some(26))
            ]
        );
    }
}
