//! 1 つのファイルの読み方（frontmatter、本文の見出しとリンク）と、行の書き換え。

use desk_core::frontmatter::Value;
use desk_core::{Doc, LinkForm, classify};

#[test]
fn parse_doc_frontmatter_and_links() {
    let content = [
        "---",
        "status: todo",
        "parent: 2026-09-07-meal-planner.md",
        "depends_on: [2026-10-14-shopping-list.md, \"2026-10-20-sync.md\"]",
        "---",
        "",
        "# 献立を印刷できるようにする",
        "",
        "## 目的",
        "[設計](../artifacts/design.md#背景) と ![図](assets/a.png)、[外](https://example.com)、[[名前]]",
        "",
        "```",
        "[コードの中](ignored.md)",
        "```",
        "## いま",
        "レビュー待ち。",
    ]
    .join("\n");
    let d = Doc::parse(classify("projects/app/tasks/2026-11-09-home.md"), &content);
    assert!(
        d.has_fm && d.fm_error.is_none(),
        "frontmatter を読めない: {:?}",
        d.fm_error
    );
    assert_eq!(d.status(), "todo");
    assert!(!d.needs_human());
    assert_eq!(
        d.list("depends_on"),
        ["2026-10-14-shopping-list.md", "2026-10-20-sync.md"]
    );
    assert_eq!(d.title(), "献立を印刷できるようにする");
    assert_eq!(d.now(), "レビュー待ち。");

    let mut links = Vec::new();
    for l in &d.links {
        links.push((l.key.as_str(), l.target.as_str(), l.form));
        if let Some(start) = l.start {
            assert!(
                d.lines[l.line][start..].starts_with(&l.raw),
                "リンク {:?} の位置がずれている（{} 行目 {} バイト目）",
                l.raw,
                l.line,
                start
            );
        }
    }
    let want = [
        (
            "parent",
            "projects/app/tasks/2026-09-07-meal-planner.md",
            LinkForm::Relative,
        ),
        (
            "depends_on",
            "projects/app/tasks/2026-10-14-shopping-list.md",
            LinkForm::Relative,
        ),
        (
            "depends_on",
            "projects/app/tasks/2026-10-20-sync.md",
            LinkForm::Relative,
        ),
        (
            "body",
            "projects/app/artifacts/design.md",
            LinkForm::Relative,
        ),
        (
            "body",
            "projects/app/tasks/assets/a.png",
            LinkForm::Relative,
        ),
        ("body", "", LinkForm::External),
        ("body", "名前", LinkForm::Wiki),
    ];
    assert_eq!(links, want);
}

#[test]
fn parse_doc_broken_frontmatter() {
    let d = Doc::parse(
        classify("projects/app/tasks/2026-11-09-x.md"),
        "---\nstatus: [active\n---\n# x\n",
    );
    assert!(d.fm_error.is_some(), "壊れた frontmatter を読めてしまう");
    let d = Doc::parse(
        classify("projects/app/tasks/2026-11-09-x.md"),
        "---\nstatus: active\n# x\n",
    );
    assert!(
        d.fm_error.is_some(),
        "閉じていない frontmatter を読めてしまう"
    );
}

#[cfg(feature = "write")]
/// 閉じる `---` の前の改行も YAML のうちなので、最後の値でも `|`・`>` の値は改行で終わる。
#[test]
fn block_value_before_closing_line_keeps_newline() {
    let d = Doc::parse(
        classify("projects/app/knowledge/x.md"),
        "---\nprinciple: |\n  a\ndescription: >\n  不具合は\n  構造で直す\n---\n# x\n",
    );
    let fm = d.frontmatter.as_ref().unwrap();
    assert_eq!(fm.get("principle").unwrap().value, Value::Str("a\n".into()));
    assert_eq!(
        fm.get("description").unwrap().value,
        Value::Str("不具合は 構造で直す\n".into())
    );
    assert_eq!(d.description(), "不具合は 構造で直す");
}

#[test]
fn apply_edits() {
    use desk_core::edit::{Edit, apply_edits};
    let lines = ["a [x](one.md) [y](two.md)".to_string(), "b".to_string()];
    let edit = |line, start, old: &str, new: &str| Edit {
        line,
        start,
        old: old.to_string(),
        new: new.to_string(),
    };
    let out = apply_edits(
        &lines,
        &[
            edit(0, 6, "one.md", "../one.md"),
            edit(0, 18, "two.md", "2.md"),
        ],
    )
    .unwrap();
    assert_eq!(out[0], "a [x](../one.md) [y](2.md)");
    assert!(
        apply_edits(&lines, &[edit(0, 0, "zzz", "q")]).is_err(),
        "置換元が違うのに書き換えられてしまう"
    );
}

/// 本文だけの文書を読む。
fn body(lines: &[&str]) -> Doc {
    Doc::parse(classify("projects/app/knowledge/a.md"), &lines.join("\n"))
}

/// 本文のリンクの (先, 行, 行の中の位置)。
fn links(d: &Doc) -> Vec<(&str, usize, Option<usize>)> {
    d.links
        .iter()
        .map(|l| (l.raw.as_str(), l.line, l.start))
        .collect()
}

/// 見出しの (段, 文字, 行)。
fn headings(d: &Doc) -> Vec<(usize, &str, usize)> {
    d.headings
        .iter()
        .map(|h| (h.level, h.text.as_str(), h.line))
        .collect()
}

/// コードと HTML の中のものはリンクでも見出しでもない。
#[test]
fn code_and_html_are_not_markdown() {
    let d = body(&[
        "`[コード](a.md)` と `` [`二重`](b.md) ``",
        "",
        "````",
        "```",
        "[フェンスの中](c.md)",
        "```",
        "# フェンスの中",
        "````",
        "",
        "    [字下げのコード](d.md)",
        "",
        "<!--",
        "[コメント](e.md)",
        "-->",
        "",
        "[外](f.md)",
    ]);
    assert_eq!(links(&d), [("f.md", 15, Some(6))]);
    assert!(d.headings.is_empty(), "{:?}", d.headings);
}

/// 見出しは CommonMark の見出しのうち、引用・リストの外のもの。文字は書いてあるとおり。
#[test]
fn headings_follow_commonmark() {
    let d = body(&[
        "  # 字下げした見出し #",
        "#見出しでない",
        "",
        "下線の見出し",
        "===",
        "",
        "## `code` と <sha> と [リンク](a.md)",
        "",
        "> ## 引用の中",
        "",
        "- ## リストの中",
        "",
        "下線の",
        "見出し 2",
        "---",
    ]);
    assert_eq!(
        headings(&d),
        [
            (1, "字下げした見出し", 0),
            (1, "下線の見出し", 3),
            (2, "`code` と <sha> と [リンク](a.md)", 6),
            (2, "下線の 見出し 2", 12),
        ]
    );
    assert_eq!(d.title(), "字下げした見出し");
}

/// リンクの形は CommonMark どおりに読む。書いてあるとおりの先なら、書き換えられる位置を持つ。
#[test]
fn links_follow_commonmark() {
    let d = body(&[
        "[入れ [子] の文字](a.md) [括弧](b(c).md) [空白](<d e.md> \"題\")",
        "[行を",
        "またぐ](f.md) \\[エスケープ](g.md) [エスケープした先](h\\_i.md)",
        "<https://example.com/x> <me@example.com>",
        "[参照][r] と [r] と [無い参照][none]",
        "",
        "[r]: r.md",
        "[r]: 2 つ目の定義.md",
        "段落の中の",
        "[s]: 段落の続き.md",
    ]);
    assert_eq!(
        links(&d),
        [
            ("a.md", 0, Some(25)),
            ("b(c).md", 0, Some(40)),
            ("d e.md", 0, Some(59)),
            ("f.md", 2, Some(11)),
            ("h_i.md", 2, None),
            ("https://example.com/x", 3, Some(1)),
            ("r.md", 6, Some(5)),
        ]
    );
    let text: Vec<&str> = d.links.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        text[..4],
        ["入れ [子] の文字", "括弧", "空白", "行を またぐ"]
    );
}

/// `[[名前]]` は Wiki の形として拾う（`|` の後ろは表示の文字）。
#[test]
fn wiki_links() {
    let d = body(&["[[名前]] と [[先|表示]]"]);
    let got: Vec<(&str, &str, LinkForm, Option<usize>)> = d
        .links
        .iter()
        .map(|l| (l.raw.as_str(), l.target.as_str(), l.form, l.start))
        .collect();
    assert_eq!(
        got,
        [
            ("[[名前]]", "名前", LinkForm::Wiki, Some(0)),
            ("[[先|表示]]", "先", LinkForm::Wiki, Some(15)),
        ]
    );
}

/// 生の HTML の a の href・img の src・source の srcset もリンクとして拾う。コードの中は拾わない。
#[test]
fn html_links() {
    let d = body(&[
        "文の中の <a href=\"a.md\">a</a> と <img",
        "src=\"assets/b.png\"> と `<a href=\"code.md\">` と [md](c.md)",
        "",
        "<div>",
        "<a",
        "  href=\"d.md#節\">d</a>",
        "<!-- <a href=\"comment.md\"> -->",
        "<picture><source srcset=\"assets/e.png 1x, assets/f.png 2x\"></picture>",
        "</div>",
        "",
        "> <p>引用の中の",
        "> <a href=\"g.md\">g</a></p>",
        "",
        "<a href=\"h&amp;i.md\" title=\"文字参照\">h</a>",
        "",
        "```",
        "<a href=\"fence.md\">",
        "```",
    ]);
    assert_eq!(
        links(&d),
        [
            ("a.md", 0, Some(22)),
            ("assets/b.png", 1, Some(5)),
            ("c.md", 1, Some(54)),
            ("d.md#節", 5, Some(8)),
            ("assets/e.png", 7, Some(25)),
            ("assets/f.png", 7, Some(42)),
            ("g.md", 11, Some(11)),
            ("h&i.md", 13, None),
        ]
    );
    let l = &d.links[3];
    assert_eq!(
        (l.form, l.target.as_str(), l.anchor.as_str()),
        (LinkForm::Relative, "projects/app/knowledge/d.md", "節")
    );
}
