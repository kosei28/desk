//! `desk check` と `desk rules` の出力の形と終了コード。何が挙がるか・どう直すかは core のテスト（core/tests/query.rs・
//! write.rs）で見るので、ここは引数が core に届くことと、表・JSON・直したものの出し方を見る。

mod common;

use std::path::Path;

use common::run::{desk, must};
use common::{check_tree, outside_tree_fails, recipe_tree, task_doc, with_git};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Finding {
    path: String,
    category: String,
    fixable: bool,
}

fn check_json(root: &Path, args: &[&str]) -> (Vec<Finding>, i32) {
    let o = desk(root, &[&["check", "--json"], args].concat());
    let fs = serde_json::from_str(&o.stdout).unwrap_or_else(|e| panic!("JSON でない: {e}\n{o:?}"));
    (fs, o.code)
}

#[test]
fn clean_tree_exits_0() {
    let f = with_git(recipe_tree());
    let o = desk(&f.root, &["check"]);
    assert!(
        o.code == 0 && o.stdout.is_empty() && o.stderr.contains("形に合っている"),
        "{o:?}"
    );
    let o = desk(&f.root, &["check", "--json"]);
    assert!(o.code == 0 && o.stdout == "[]\n", "{o:?}");
}

#[test]
fn findings_as_table_and_json() {
    let f = with_git(check_tree());
    let (fs, code) = check_json(&f.root, &[]);
    assert!(code == 1 && !fs.is_empty(), "code={code}: {fs:?}");

    let o = desk(&f.root, &["check"]);
    assert_eq!(o.code, 1, "{o:?}");
    let mut lines = o.stdout.lines();
    let header: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(header, ["path", "何を", "何が", "どう直すか"]);
    assert_eq!(
        lines.count(),
        fs.len(),
        "表と JSON で件数が違う:\n{}",
        o.stdout
    );
    let fixable = fs.iter().filter(|x| x.fixable).count();
    assert!(
        o.stderr
            .contains(&format!("{} 件中 {fixable} 件は --fix で直せる", fs.len())),
        "{}",
        o.stderr
    );
    assert!(fs.iter().any(|x| x.category == "frontmatter"), "{fs:?}");
}

/// `--name`・`--project` が core の絞り込みに届く。
#[test]
fn narrowing_reaches_core() {
    let f = with_git(check_tree());
    let p = |s: &str| format!("projects/recipe-app/{s}");
    let (fs, _) = check_json(&f.root, &["--name", "2026-11-12-todo-parent"]);
    assert!(!fs.is_empty());
    for x in &fs {
        assert!(
            x.path == p("tasks/2026-11-12-todo-parent.md")
                || x.path == p("tasks/2026-11-12-started-child.md"),
            "--name で関係ないものが出る: {x:?}"
        );
    }
    let (fs, code) = check_json(&f.root, &["--project", "misc"]);
    assert!(
        code == 0 && fs.is_empty(),
        "--project misc で他が出る: {fs:?}"
    );
}

/// `--fix` は直したファイルとコミットのしかたを出す。直せないものが残れば 1。
#[test]
fn fix_reports_changes() {
    let f = with_git(recipe_tree());
    f.write(
        "projects/recipe-app/tasks/2026-11-04-closed.md",
        &task_doc("done", "", "閉じた"),
    );
    let o = desk(&f.root, &["check", "--fix"]);
    let moved = "projects/recipe-app/archive/tasks/2026-11-04-closed.md";
    assert_eq!(o.code, 0, "{o:?}");
    for want in ["直した:", moved, "git -C"] {
        assert!(
            o.stdout.contains(want),
            "出力に {want:?} が無い:\n{}",
            o.stdout
        );
    }

    f.write("projects/recipe-app/knowledge/empty.md", "# 中身だけ\n");
    let o = desk(&f.root, &["check", "--fix", "--json"]);
    let fs: Vec<Finding> = serde_json::from_str(&o.stdout).unwrap();
    assert!(
        o.code == 1 && !fs.is_empty(),
        "手で直すもの（description・principle が空）が残るのに: {o:?}"
    );
}

/// `desk rules` は core の形の一覧をそのまま出す（中身は core のテストで見る）。ツリーの外でも動く。
#[test]
fn rules_prints_core_description() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(must(dir.path(), &["rules"]), desk_core::rules::describe());
}

#[test]
fn check_outside_tree_fails() {
    outside_tree_fails(&[&["check"], &["check", "--fix"]]);
}
