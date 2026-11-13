//! `desk init`・`root`・`new` の引数と出力。何を作るか（ファイルの中身・名前の決まり）は core のテスト
//! （core/tests/write.rs）で見るので、ここは引数が core に届くことと、出し方・終了コードを見る。

mod common;

use std::fs;
use std::path::PathBuf;

use common::run::{desk, must};
use common::{exists, git, outside_tree_fails, read};
use tempfile::TempDir;

/// シンボリックリンクを解いた一時ディレクトリ。
fn temp_dir() -> (TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().canonicalize().unwrap();
    (d, p)
}

/// desk init で作ったツリー。
fn new_tree() -> (TempDir, PathBuf) {
    let (d, base) = temp_dir();
    let root = base.join("desk");
    must(&base, &["init", root.to_str().unwrap()]);
    (d, root)
}

#[test]
fn init_reports_what_it_did() {
    let (_d, base) = temp_dir();
    let root = base.join("notes");
    let out = must(&base, &["init", root.to_str().unwrap()]);
    for want in ["を作った:", "  .desk", "最初のコミットを作った"] {
        assert!(out.contains(want), "{want:?} が無い:\n{out}");
    }
    let out = must(&base, &["init", root.to_str().unwrap()]);
    assert!(out.contains("もうツリーのルート"), "2 回目:\n{out}");

    // 省略は作業ディレクトリ
    let (_d, dir) = temp_dir();
    must(&dir, &["init"]);
    assert!(exists(&dir, ".desk"), "作業ディレクトリに作られていない");

    // あるリポジトリにはコミットせず、add のしかたを出す
    let (_d, dir) = temp_dir();
    fs::write(dir.join("README.md"), "# 前からある\n").unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["add", "README.md"]);
    git(&dir, &["commit", "-q", "-m", "first"]);
    let out = must(&dir, &["init"]);
    assert!(out.contains("コミットはしていない: git -C"), "{out}");

    // ツリーの中には作らない
    let o = desk(&root, &["init", "projects/misc/nested"]);
    assert!(
        o.code == 1 && o.stderr.contains("ツリーの中にツリーは作らない"),
        "{o:?}"
    );
}

#[test]
fn root_discovery() {
    let (_d, root) = new_tree();
    let sub = root.join("projects/misc");
    assert_eq!(must(&sub, &["root"]).trim(), root.to_str().unwrap());
    let got = must(&sub, &["read", "projects/misc/README.md"]);
    assert!(
        got.contains("# misc"),
        "サブディレクトリから読めない: {got:?}"
    );
}

#[test]
fn outside_tree_fails_for_new() {
    outside_tree_fails(&[
        &["root"],
        &["read", "x.md"],
        &["new", "project", "recipe-app"],
        &["new", "task", "x", "--project", "misc"],
    ]);
}

/// 作ったファイルの絶対パスを出す。引数は core に届く。
#[test]
fn new_prints_created_path() {
    let (_d, root) = new_tree();
    let out = must(&root, &["new", "project", "recipe-app"]);
    assert_eq!(
        out,
        format!("{}\n", root.join("projects/recipe-app/README.md").display())
    );

    let parent = "projects/recipe-app/tasks/2026-09-07-meal-planner.md";
    let out = must(
        &root,
        &[
            "new",
            "task",
            "2026-09-07-meal-planner",
            "--project",
            "recipe-app",
            "--title",
            "献立プランナー",
        ],
    );
    assert_eq!(out, format!("{}\n", root.join(parent).display()));
    assert!(read(&root, parent).contains("# 献立プランナー"));

    must(
        &root,
        &[
            "new",
            "task",
            "2026-11-10-decide-paper-size",
            "--parent",
            "2026-09-07-meal-planner",
            "--needs-human",
        ],
    );
    let child = read(
        &root,
        "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md",
    );
    assert!(
        child.contains("parent: 2026-09-07-meal-planner.md") && child.contains("needs_human: true"),
        "{child}"
    );

    // 作れないものは止まり、何も作らない
    let o = desk(&root, &["new", "task", "x", "--project", "nothing"]);
    assert!(o.code != 0 && o.stderr.starts_with("desk: "), "{o:?}");
    let o = desk(&root, &["new", "project", "recipe-app"]);
    assert!(o.code != 0 && o.stderr.starts_with("desk: "), "{o:?}");
    assert!(!exists(&root, "projects/nothing"));
}
