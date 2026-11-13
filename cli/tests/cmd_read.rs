//! `desk read` の引数と出力の形（段・outline・section、名前で引く、`-` で標準入力から読む）。

mod common;

use common::run::{desk, must};
use common::{recipe_tree, with_git};

#[test]
fn read_levels() {
    let f = with_git(recipe_tree());
    let task = "projects/recipe-app/tasks/2026-11-09-printable-menu.md";

    let out = must(&f.root, &["read", task, "--level", "0"]);
    assert!(
        out.contains("[task · recipe-app · inprogress]") && out.contains("いま: いま。"),
        "段 0:\n{out}"
    );

    let out = must(
        &f.root,
        &[
            "read",
            "2026-11-09-printable-menu",
            "--section",
            "目的,いま",
        ],
    );
    assert!(
        out.contains("## 目的") && out.contains("## いま") && !out.contains("## ログ"),
        "節で切れていない:\n{out}"
    );

    let out = must(&f.root, &["read", task, "--outline"]);
    assert!(
        out.contains("ログ") && !out.contains("印刷用のページ"),
        "見出しだけになっていない:\n{out}"
    );

    let out = must(
        &f.root,
        &[
            "read",
            "projects/recipe-app/wiki/meal-planner/overview.md",
            "--level",
            "2",
        ],
    );
    assert!(
        out.contains("--- 指す先 ---")
            && out.contains("==> projects/recipe-app/artifacts/print-layout.md <=="),
        "段 2 で指す先を読まない:\n{out}"
    );
}

#[test]
fn read_ambiguous_name() {
    let f = with_git(recipe_tree());
    f.write(
        "projects/misc/artifacts/print-layout.md",
        "---\ntasks: []\n---\n# 別の\n",
    );
    let o = desk(&f.root, &["read", "print-layout"]);
    assert!(
        o.code != 0 && o.stderr.contains("複数ある"),
        "名前が複数に当たるのに止まらない: {o:?}"
    );
    let out = must(
        &f.path("projects/recipe-app"),
        &["read", "print-layout", "--level", "0"],
    );
    assert!(
        out.contains("projects/recipe-app/artifacts/print-layout.md"),
        "作業ディレクトリのプロジェクトで絞れていない:\n{out}"
    );
}

/// `-` は標準入力の 1 行を 1 つのパスとして読む（空の行と前後の空白は捨てる）。ほかのパスと並べてもよい。
#[test]
fn read_paths_from_stdin() {
    let f = with_git(recipe_tree());
    let paths = must(&f.root, &["ls", "task", "--paths"]);
    let o = common::run::desk_with_stdin(
        &f.root,
        &["read", "README.md", "-", "--level", "0"],
        &format!("\n  {paths}\n"),
    );
    assert_eq!(o.code, 0, "{o:?}");
    assert!(o.stdout.starts_with("README.md  ["), "{}", o.stdout);
    for p in paths.lines() {
        assert!(
            o.stdout.contains(&format!("{p}  [task")),
            "{p} を読まない:\n{}",
            o.stdout
        );
    }
}
