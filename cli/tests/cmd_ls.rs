//! `desk ls` の引数と出力の形。何に当たるか（`--where` の読み方など）は core のテスト（core/tests/query.rs）で見る
//! ので、ここは引数が core に届くことと、表・JSON・パス・段の出し方を見る。

mod common;

use common::run::must;
use common::{outside_tree_fails, recipe_tree, task_doc, with_git};
use serde_json::Value;

const MENU: &str = "projects/recipe-app/tasks/2026-11-09-printable-menu.md";

#[test]
fn arguments_reach_core() {
    let f = with_git(recipe_tree());
    f.write(
        "projects/recipe-app/archive/tasks/2026-09-07-favorites.md",
        &task_doc("done", "", "お気に入り"),
    );
    let ls = |args: &[&str]| must(&f.root, &[&["ls"], args].concat());

    let out = ls(&[
        "task",
        "--where",
        "parent=2026-09-07-meal-planner",
        "--paths",
    ]);
    assert_eq!(out, format!("{MENU}\n"), "--where");
    let out = ls(&["task", "--archived", "--paths"]);
    assert!(
        out.contains("archive/tasks/2026-09-07-favorites.md"),
        "--archived:\n{out}"
    );
    let out = ls(&["principle", "--project", "recipe-app", "--paths"]);
    assert_eq!(
        out, "principles/prefer-structure-over-patches.md\n",
        "--project"
    );
    let out = ls(&["--name", "2026-11-09-printable-menu", "--paths"]);
    assert_eq!(out, format!("{MENU}\n"), "--name");

    // 種類はカンマ区切りでも、並べても
    let kinds = ls(&["task,wiki", "--paths"]);
    assert_eq!(ls(&["task", "wiki", "--paths"]), kinds);
    assert!(
        kinds.contains(MENU) && kinds.contains("wiki/meal-planner/overview.md"),
        "{kinds}"
    );

    // 最終コミットの日（コミットしていないものは今日）。日数でも日付でも
    let all = ls(&["task", "--paths"]);
    assert_eq!(ls(&["task", "--modified-after", "0", "--paths"]), all);
    assert_eq!(
        ls(&["task", "--modified-after", "2000-01-01", "--paths"]),
        all
    );
    assert_eq!(
        ls(&["task", "--modified-before", "2000-01-01", "--paths"]),
        ""
    );
}

#[test]
fn output_forms() {
    let f = with_git(recipe_tree());
    let ls = |args: &[&str]| must(&f.root, &[&["ls"], args].concat());

    // 表: 種類を 1 つに絞ると、その種類の列
    let out = ls(&["task"]);
    let header: Vec<&str> = out.lines().next().unwrap().split_whitespace().collect();
    assert_eq!(header, ["path", "status", "needs_human", "now"]);
    // --fields はカンマ区切りでも、繰り返しても
    let out = ls(&["task", "--fields", "path", "--fields", "status,title"]);
    let header: Vec<&str> = out.lines().next().unwrap().split_whitespace().collect();
    assert_eq!(header, ["path", "status", "title"]);

    // JSON: --fields の列も足す
    let rows: Value =
        serde_json::from_str(&ls(&["wiki", "--json", "--fields", "description"])).unwrap();
    let rows = rows.as_array().unwrap();
    assert!(
        rows.len() == 1
            && rows[0]["path"] == "projects/recipe-app/wiki/meal-planner/overview.md"
            && rows[0]["description"].is_string(),
        "--json: {rows:?}"
    );
    // キーは決まった順で、frontmatter は書いてあるとおりの順で、--fields の列は後ろに
    let keys = |v: &Value| v.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        keys(&rows[0]),
        [
            "path",
            "name",
            "kind",
            "project",
            "archived",
            "title",
            "frontmatter",
            "description"
        ]
    );
    let rows: Value = serde_json::from_str(&ls(&["task", "--json"])).unwrap();
    let task = &rows.as_array().unwrap()[0];
    assert_eq!(keys(task)[6], "now", "{task}");
    assert_eq!(
        keys(&task["frontmatter"])[..2],
        ["status", "needs_human"],
        "{task}"
    );

    // 段: プロジェクトは README.md を読む
    let out = ls(&["project", "--level", "0"]);
    assert!(
        out.contains("projects/recipe-app/README.md  [")
            && out.contains("projects/misc/README.md  ["),
        "{out}"
    );
}

#[test]
fn move_and_list_outside_tree_fail() {
    outside_tree_fails(&[
        &["ls"],
        &["graph", "--all"],
        &["archive", "x.md"],
        &["mv", "a.md", "b.md"],
    ]);
}
