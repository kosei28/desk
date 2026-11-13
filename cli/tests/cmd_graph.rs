//! `desk graph` の引数と出力の形。どの辺をたどるかは core のテスト
//! （core/tests/query.rs）で見るので、ここは引数が core に届くことと、出し方を見る。

mod common;

use common::run::{desk, must};
use common::{recipe_tree, task_doc, with_git};

#[test]
fn graph() {
    let f = with_git(recipe_tree());
    f.write(
        "projects/recipe-app/tasks/2026-11-11-print-preview.md",
        &task_doc(
            "todo",
            "parent: 2026-11-09-printable-menu.md",
            "印刷のプレビューを出す",
        ),
    );

    let out = must(
        &f.root,
        &[
            "graph",
            "2026-09-07-meal-planner",
            "--in",
            "--via",
            "parent",
            "--depth",
            "all",
            "--format",
            "paths",
        ],
    );
    assert_eq!(
        out,
        "projects/recipe-app/tasks/2026-11-09-printable-menu.md\nprojects/recipe-app/tasks/2026-11-11-print-preview.md\n",
        "子孫"
    );

    let out = must(
        &f.root,
        &[
            "graph",
            "2026-11-09-printable-menu",
            "--in",
            "--format",
            "paths",
        ],
    );
    assert!(
        out.contains("artifacts/print-layout.md") && out.contains("2026-11-11-print-preview.md"),
        "指しているもの:\n{out}"
    );

    let out = must(
        &f.root,
        &[
            "graph",
            "projects/recipe-app/artifacts/print-layout.md",
            "--out",
        ],
    );
    assert!(
        out.contains("→ tasks  projects/recipe-app/tasks/2026-11-09-printable-menu.md")
            && out.contains("→ body  projects/recipe-app/artifacts/assets/print-sample.png"),
        "指す先:\n{out}"
    );

    let out = must(&f.root, &["graph", "--all", "--format", "dot"]);
    assert!(
        out.starts_with("digraph desk {") && out.contains(r#"[label="principle"]"#),
        "dot:\n{out}"
    );

    assert_ne!(
        desk(&f.root, &["graph", "x", "--in", "--out"]).code,
        0,
        "--in と --out を両方付けて通る"
    );
}
