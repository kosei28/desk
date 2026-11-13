//! `desk archive`・`mv` の引数と出力。どこへ動き、どのリンクをどう付け替えるかは core のテスト
//! （core/tests/write.rs）で見るので、ここは引数が core に届くことと、計画・結果の出し方を見る。

mod common;

use common::run::{desk, must};
use common::{exists, git_add_all, recipe_tree, with_git};

const TASK: &str = "projects/recipe-app/tasks/2026-11-09-printable-menu.md";
const MOVED: &str = "projects/recipe-app/archive/tasks/2026-11-09-printable-menu.md";

#[test]
fn archive_prints_plan_and_result() {
    let f = with_git(recipe_tree());
    let root = &f.root;
    git_add_all(root);
    let args = [
        "archive",
        TASK,
        "projects/recipe-app/artifacts/print-layout.md",
        "--status",
        "dropped",
    ];

    let out = must(root, &[&args[..], &["--dry-run"]].concat());
    assert!(exists(root, TASK), "--dry-run で動いた");
    for want in [
        "動かす:",
        &format!("  {TASK} → {MOVED}"),
        "書き換える:",
        &format!("  {TASK}:2  inprogress → dropped"),
    ] {
        assert!(out.contains(want), "計画に {want:?} が無い:\n{out}");
    }

    let out = must(root, &args);
    assert!(!exists(root, TASK) && exists(root, MOVED), "動いていない");
    for want in [
        "動かした:",
        &format!("  {TASK} → {MOVED}"),
        "書き換えた（リンク・status）:",
        "コミットするとき: git -C ",
    ] {
        assert!(out.contains(want), "結果に {want:?} が無い:\n{out}");
    }

    let out = must(root, &["archive", MOVED]);
    assert_eq!(out, format!("既に archive/ にある: {MOVED}\n"));
    let o = desk(
        root,
        &[
            "archive",
            "projects/recipe-app/wiki/meal-planner/overview.md",
        ],
    );
    assert!(
        o.code != 0 && o.stderr.starts_with("desk: "),
        "wiki を移せてしまう: {o:?}"
    );
}

#[test]
fn archive_project() {
    let f = with_git(recipe_tree());
    let out = must(&f.root, &["archive", "--project", "recipe-app"]);
    assert!(
        out.contains("  projects/recipe-app/README.md → archive/projects/recipe-app/README.md"),
        "{out}"
    );
    assert!(!exists(&f.root, "projects/recipe-app"));
}

#[test]
fn mv_rename_and_refuse() {
    let f = with_git(recipe_tree());
    let root = &f.root;
    let from = "projects/recipe-app/artifacts/print-layout.md";
    let to = "projects/recipe-app/artifacts/print-columns.md";
    let out = must(root, &["mv", from, to]);
    assert!(out.contains(&format!("  {from} → {to}")), "{out}");
    assert!(exists(root, to));

    let o = desk(root, &["mv", to, "projects/recipe-app/drafts/"]);
    assert!(
        o.code != 0 && o.stderr.contains("決まりの外"),
        "決まりの外へ動かせてしまう: {o:?}"
    );
}
