//! Go の cli/internal/desk/doc_test.go を移したもの。

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
