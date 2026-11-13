//! 差分テスト（R2）: 引く・読む・確かめる。同じ架空のツリーで、Go の CLI と、core を呼んで同じ形に整えたもの
//! （tests/common/cli.rs）を流し、標準出力・標準エラー・終了コードを比べる。
//! Go の CLI を消す切り替え（R5）で、Go と一緒に消す。
//!
//! - `ls`: `--paths`・`--json`（`--fields` も）・`--level`。条件は `--where` の全部の演算子、`--name`・`--project`・
//!   `--archived`・`--modified-before`・`--modified-after`
//! - `graph`: `--format tree|paths|dot`、`--in`・`--out`・`--via`・`--depth`・`--level`、`--all --format dot`
//! - `read`: `--level 0|1|2`・`--outline`・`--section`、複数のファイル、名前での指定と作業ディレクトリ
//! - `check`: `--json`（`--name`・`--project`）と `--rules`
//!
//! 表（`ls`・`graph --all`・`check` の既定の出力）は CLI の段（R4）で、`check --fix` は書く側（R3）で比べる。
//!
//! Go が要る。`go` が無ければ失敗する（Go を入れずに飛ばすなら DESK_SKIP_GO_DIFF=1）。

mod common;

use std::path::Path;

use common::cli;
use common::go::{fix_go_fm_lines, fix_go_yaml_errors, go_desk, go_run};
use common::{Fixture, check_tree, edge_tree, recipe_tree, task_doc};
use desk_core::Tree;
use desk_core::query::days_before;

/// cwd で両方を流し、全部同じか見る。
fn same_in(cwd: &Path, args: &[&str]) {
    let want = go_run(cwd, args);
    let got = cli::run(cwd, args);
    assert_eq!(got, want, "desk {} が Go と違う", args.join(" "));
}

fn same_all(f: &Fixture, cases: &[&[&str]]) {
    if go_desk().is_none() {
        return;
    }
    for args in cases {
        same_in(&f.root, args);
    }
}

/// 引くと読むのテストのツリー（Go の TestLsFilters・TestGraph・TestKnowledgeWithSeveralPrinciples のツリーを合わせた）。
fn query_tree() -> Fixture {
    let f = recipe_tree();
    f.write(
        "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md",
        "---\nstatus: todo\nneeds_human: true\nparent: 2026-11-09-printable-menu.md\n---\n\n# 用紙の大きさを決める\n\n## 目的\n選ぶ。\n\n## 成果物\n- なし\n",
    );
    f.write(
        "projects/recipe-app/archive/tasks/2026-09-07-favorites.md",
        &task_doc("done", "", "お気に入り"),
    );
    f.write(
        "projects/recipe-app/tasks/2026-11-11-print-preview.md",
        &task_doc(
            "todo",
            "parent: 2026-11-09-printable-menu.md\ndepends_on: [2026-11-10-decide-paper-size.md]",
            "印刷のプレビューを出す",
        ),
    );
    f.write(
        "principles/single-source-of-truth.md",
        "---\ndescription: 1 つの事実は 1 か所に置く\n---\n\n# 事実は 1 か所\n",
    );
    f.write(
        "knowledge/derive-state-from-input.md",
        "---\ndescription: 画面の状態を直すときに読む。状態を入力から導く\nprinciple: [../principles/prefer-structure-over-patches.md, ../principles/single-source-of-truth.md]\nsources: []\nverified: 2026-11-12\n---\n\n# 状態は入力から導く\n\n手順は [詳細](derive-state-steps.md)。[タスク](../projects/recipe-app/tasks/2026-11-09-printable-menu.md)\n",
    );
    f.write(
        "knowledge/derive-state-steps.md",
        "---\ndescription: 導き方の手順\n---\n\n# 導き方の手順\n\n[戻る](derive-state-from-input.md)\n",
    );
    f.write("projects/misc/inbox/2026-11-12-memo.md", "# メモ\n\n- a\n");
    f.write(
        "projects/recipe-app/library/2026-11-12-scroll.md",
        "---\ntags: [frontend, pagination]\nestimate: 9\n---\n\n# 無限スクロール\n",
    );
    f.write(
        "projects/recipe-app/library/2026-11-13-sizes.md",
        "---\ntags: [print]\nestimate: 10\n---\n\n# 用紙\n",
    );
    f
}

#[test]
fn ls_matches_go() {
    same_all(
        &query_tree(),
        &[
            &["ls", "--paths"],
            &["ls", "--archived", "--paths"],
            &["ls", "task", "--paths"],
            &["ls", "task,artifact", "--archived", "--paths"],
            &["ls", "task", "--where", "needs_human=true", "--paths"],
            &[
                "ls",
                "task",
                "--where",
                "parent=2026-09-07-meal-planner",
                "--paths",
            ],
            &[
                "ls",
                "task",
                "--where",
                "parent=2026-09-07-meal-planner.md",
                "--paths",
            ],
            &[
                "ls",
                "task",
                "--where",
                "parent=projects/recipe-app/tasks/2026-09-07-meal-planner.md",
                "--paths",
            ],
            &["ls", "--where", "parent=", "--paths"],
            &["ls", "--where", "parent!=[]", "--paths"],
            &[
                "ls",
                "artifact",
                "--where",
                "tasks~printable-menu",
                "--paths",
            ],
            &["ls", "knowledge", "--where", "sources=[]", "--paths"],
            &[
                "ls",
                "knowledge",
                "--where",
                "principle=single-source-of-truth",
                "--paths",
            ],
            &["ls", "principle", "--project", "recipe-app", "--paths"],
            &["ls", "knowledge", "--project", "misc", "--paths"],
            &["ls", "project", "--paths"],
            &["ls", "project", "--name", "recipe-app", "--paths"],
            &["ls", "--name", "print-layout", "--paths"],
            &["ls", "--name", "print-layout.md", "--paths"],
            &[
                "ls",
                "task",
                "--where",
                "status!=inprogress",
                "--where",
                "needs_human=false",
                "--paths",
            ],
            &["ls", "--where", "kind=wiki", "--paths"],
            &["ls", "--where", "title~献立,用紙", "--paths"],
            &["ls", "--where", "tags~print,front", "--paths"],
            &["ls", "--where", "path>projects/recipe-app/tasks", "--paths"],
            &["ls", "--where", "estimate<10", "--paths"],
            &["ls", "--where", "estimate>9", "--paths"],
            &["ls", "--where", "name<2026-11-10", "--paths"],
            &["ls", "--where", "now~いま", "--paths"],
            &["ls", "--where", "project=misc", "--paths"],
            &["ls", "--json"],
            &["ls", "--archived", "--json"],
            &[
                "ls",
                "task",
                "--json",
                "--fields",
                "status,needs_human,now,parent,modified,description,nothing,name",
            ],
            &["ls", "wiki", "--json", "--fields", "description"],
            &["ls", "library", "--json", "--fields", "tags,estimate"],
            &["ls", "project", "--json", "--fields", "name,title,modified"],
            &["ls", "--where", "nothing=x", "--json"],
            &["ls", "task", "--level", "0"],
            &["ls", "task", "--level", "1"],
            &["ls", "principle", "--project", "recipe-app", "--level", "1"],
            &["ls", "knowledge", "--level", "2"],
            &["ls", "project", "--level", "0"],
            &["ls", "project", "--level", "1"],
            &["ls", "--modified-before", "0", "--paths"],
            &["ls", "--modified-after", "0", "--paths"],
            // 使い方の誤り
            &["ls", "--where", "=x", "--paths"],
            &["ls", "--where", "a?b", "--paths"],
            &["ls", "--where", "a\u{a0}b", "--paths"],
            &["ls", "nothing"],
        ],
    );
}

#[test]
fn ls_on_edge_tree_matches_go() {
    same_all(
        &edge_tree(),
        &[
            &["ls", "--archived", "--paths"],
            &[
                "ls",
                "--archived",
                "--json",
                "--fields",
                "parent,depends_on,tags,pr,empty,description",
            ],
            &[
                "ls",
                "task",
                "--where",
                "depends_on=2026-11-02-human",
                "--paths",
            ],
            &[
                "ls",
                "task",
                "--where",
                "depends_on=projects/alpha/tasks/2026-11-03-broken.md",
                "--paths",
            ],
            &["ls", "--where", "tilde=", "--paths"],
            &["ls", "--where", "quoted_null=null", "--paths"],
            &["ls", "--where", "estimate>1.4", "--paths"],
            &["ls", "--where", "pr~pr/1", "--paths"],
            &[
                "ls",
                "--where",
                "knowledge=../../../knowledge/review.md",
                "--paths",
            ],
            &["ls", "--where", "knowledge=knowledge/review.md", "--paths"],
            &["ls", "--where", "source=2026-11-01-main", "--paths"],
            &["ls", "--archived", "--level", "0"],
            &["ls", "--project", "alpha", "--level", "2"],
            &["ls", "project", "--archived", "--level", "0"],
        ],
    );
}

/// 最終コミットの日付で絞る（コミットされていないものは今日として比べる）。
#[test]
fn ls_modified_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let today = String::from_utf8(
        std::process::Command::new("date")
            .arg("+%F")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let f = query_tree();
    f.commit_all(&days_before(&today, 30).unwrap());
    f.write(
        "projects/recipe-app/tasks/2026-09-07-meal-planner.md",
        &task_doc("inprogress", "", "献立プランナー（直した）"),
    );
    f.write(
        "projects/recipe-app/README.md",
        "# recipe-app\n\n直した。\n",
    );
    f.commit_all(&days_before(&today, 5).unwrap());
    f.write(
        "projects/misc/inbox/2026-11-14-new.md",
        "# まだコミットしていない\n",
    );
    let before10 = format!("modified<{}", days_before(&today, 10).unwrap());
    for args in [
        &["ls", "--modified-before", "10", "--paths"][..],
        &["ls", "--modified-after", "10", "--paths"],
        &["ls", "--modified-after", "5", "--paths"],
        &["ls", "--modified-after", "4", "--paths"],
        &["ls", "--modified-before", "40", "--paths"],
        &[
            "ls",
            "--modified-before",
            "0",
            "--modified-after",
            "31",
            "--paths",
        ],
        &["ls", "--where", &before10, "--paths"],
        &["ls", "--where", "modified=", "--paths"],
        &["ls", "--json", "--fields", "modified"],
        &["ls", "project", "--json", "--fields", "modified"],
    ] {
        same_in(&f.root, args);
    }
}

#[test]
fn graph_matches_go() {
    same_all(
        &query_tree(),
        &[
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
            &[
                "graph",
                "2026-11-09-printable-menu",
                "--in",
                "--format",
                "paths",
            ],
            &[
                "graph",
                "projects/recipe-app/artifacts/print-layout.md",
                "--out",
            ],
            &["graph", "2026-11-09-printable-menu"],
            &["graph", "2026-11-09-printable-menu", "--depth", "all"],
            &["graph", "2026-09-07-meal-planner", "--depth", "2"],
            &[
                "graph",
                "2026-09-07-meal-planner",
                "--depth",
                "2",
                "--format",
                "dot",
            ],
            &[
                "graph",
                "2026-11-09-printable-menu",
                "--via",
                "parent,body",
                "--depth",
                "3",
            ],
            &["graph", "derive-state-from-input", "--depth", "all"],
            &[
                "graph",
                "prefer-structure-over-patches",
                "--in",
                "--via",
                "principle",
                "--format",
                "paths",
            ],
            &[
                "graph",
                "projects/recipe-app/artifacts/assets/print-sample.png",
                "--in",
            ],
            &["graph", "2026-11-09-printable-menu", "--level", "0"],
            &["graph", "2026-11-09-printable-menu", "--in", "--level", "1"],
            &[
                "graph",
                "2026-11-11-print-preview",
                "--out",
                "--depth",
                "all",
                "--level",
                "2",
            ],
            &["graph", "--all", "--format", "dot"],
            &["graph", "--all", "--via", "tasks,body", "--format", "dot"],
            // 使い方の誤りと、引けないもの
            &["graph", "x", "--in", "--out"],
            &["graph", "nothing"],
            &["graph", "projects/recipe-app"],
            &["graph", "2026-11-09-printable-menu", "--depth", "0"],
            &["graph", "2026-11-09-printable-menu", "--format", "edges"],
        ],
    );
}

#[test]
fn graph_on_edge_tree_matches_go() {
    same_all(
        &edge_tree(),
        &[
            &["graph", "2026-11-01-main", "--depth", "all"],
            &[
                "graph",
                "2026-11-01-main",
                "--depth",
                "all",
                "--format",
                "dot",
            ],
            &["graph", "guide", "--in", "--depth", "all"],
            &["graph", "review", "--depth", "all", "--level", "0"],
            &["graph", "--all", "--format", "dot"],
        ],
    );
}

#[test]
fn read_matches_go() {
    let task = "projects/recipe-app/tasks/2026-11-09-printable-menu.md";
    same_all(
        &query_tree(),
        &[
            &["read", task],
            &["read", task, "--level", "0"],
            &["read", task, "--level", "2"],
            &[
                "read",
                "2026-11-09-printable-menu",
                "--section",
                "目的,いま",
            ],
            &[
                "read",
                "2026-11-09-printable-menu",
                "--section",
                "ログ,無い節,手順",
            ],
            &["read", task, "--outline"],
            &[
                "read",
                "projects/recipe-app/wiki/meal-planner/overview.md",
                "--level",
                "2",
            ],
            &["read", "derive-state-from-input", "--level", "2"],
            &["read", "2026-11-10-decide-paper-size", "--level", "0"],
            &["read", "2026-09-07-favorites", "--level", "0"],
            &["read", "projects/recipe-app/README.md", "--level", "0"],
            &["read", "2026-11-12-memo", "--level", "0"],
            &["read", "2026-11-12-memo", "--outline"],
            &["read", "print-sample.png"],
            &["read", "print-sample.png", "--level", "0"],
            &["read", "print-sample.png", "--outline"],
            &["read", "print-sample.png", "--level", "2"],
            &["read", task, "print-layout", "single-source-of-truth"],
            &["read", task, "print-layout", "--level", "0"],
            &["read", task, "print-layout", "--outline"],
            &["read", task, "print-layout", "--section", "目的"],
            &["read", task, "print-layout", "--level", "2"],
            &[
                "read",
                "./projects/recipe-app/../recipe-app/artifacts/print-layout.md",
                "--level",
                "0",
            ],
            // 引けないもの
            &["read", "nothing"],
            &["read", "projects/nothing.md"],
            &["read", "projects/recipe-app"],
            &["read", task, "--level", "3"],
        ],
    );
}

#[test]
fn read_on_edge_tree_matches_go() {
    same_all(
        &edge_tree(),
        &[
            &["read", "2026-11-01-main"],
            &["read", "2026-11-01-main", "--level", "0"],
            &["read", "2026-11-01-main", "--level", "2"],
            &["read", "2026-11-01-main", "--outline"],
            &["read", "2026-11-01-main", "--section", "目的,いま"],
            &["read", "2026-11-02-human", "--level", "0"],
            &["read", "2026-11-03-broken", "--level", "0"],
            &["read", "2026-11-04-unclosed", "--outline"],
            &["read", "2026-11-06-dots", "--outline"],
            &["read", "projects/alpha/README.md", "--level", "0"],
            &["read", "2026-11-06-bare", "--level", "0"],
            &["read", "review", "--level", "2"],
            &["read", "nested", "--level", "0"],
        ],
    );
}

/// 名前が複数に当たるときと、作業ディレクトリのプロジェクトで絞るとき（Go の TestReadAmbiguousName）。
#[test]
fn resolve_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = query_tree();
    f.write(
        "projects/misc/artifacts/print-layout.md",
        "---\ntasks: []\n---\n# 別の\n",
    );
    let args: &[&[&str]] = &[
        &["read", "print-layout", "--level", "0"],
        &["read", "print-layout.md", "--level", "0"],
        &["read", "artifacts/print-layout.md", "--level", "0"],
        &[
            "read",
            "../recipe-app/artifacts/print-layout.md",
            "--level",
            "0",
        ],
        &["read", "../../README.md", "--level", "0"],
        &["read", "~/nothing.md", "--level", "0"],
        &["graph", "print-layout", "--format", "paths"],
        &["check", "--json", "--name", "print-layout"],
    ];
    for cwd in [
        f.root.clone(),
        f.path("projects/recipe-app"),
        f.path("projects/recipe-app/artifacts"),
        f.path("projects/misc"),
        f.path("projects"),
    ] {
        for a in args {
            same_in(&cwd, a);
        }
    }
    // ルートの外の絶対パス
    let outside = f.root.parent().unwrap().join("outside.md");
    std::fs::write(&outside, "# 外\n").unwrap();
    same_in(&f.root, &["read", outside.to_str().unwrap()]);
    let inside = f.path("projects/recipe-app/artifacts/print-layout.md");
    same_in(&f.root, &["read", inside.to_str().unwrap(), "--level", "0"]);
}

#[test]
fn check_rules_matches_go() {
    same_all(&Fixture::new(), &[&["check", "--rules"]]);
}

#[test]
fn check_clean_trees_match_go() {
    if go_desk().is_none() {
        return;
    }
    for f in [query_tree(), recipe_tree(), Fixture::new()] {
        same_in(&f.root, &["check", "--json"]);
    }
}

/// check --json を比べる。Go と変えたところ（frontmatter の値の行、YAML のエラーの文言）は Go の側を直してから比べ、
/// 直した数を返す。
fn same_check(f: &Fixture, cwd: &Path, args: &[&str]) -> (usize, usize) {
    let mut want = go_run(cwd, args);
    let got = cli::run(cwd, args);
    let lines = fix_go_fm_lines(&f.root, &mut want);
    let yaml = fix_go_yaml_errors(&f.root, &mut want, &got);
    assert_eq!(got, want, "desk {} が Go と違う", args.join(" "));
    (lines, yaml)
}

#[test]
fn check_findings_match_go() {
    if go_desk().is_none() {
        return;
    }
    let f = check_tree();
    let (lines, _) = same_check(&f, &f.root, &["check", "--json"]);
    assert!(lines > 0, "frontmatter の値の行の例外を確かめていない");
    for args in [
        &["check", "--json", "--name", "2026-11-08-child"][..],
        &["check", "--json", "--name", "2026-11-16-a"],
        &[
            "check",
            "--json",
            "--name",
            "projects/recipe-app/artifacts/links.md",
        ],
        &["check", "--json", "--name", "2026-11-07-dep"],
        &["check", "--json", "--project", "misc"],
        &["check", "--json", "--project", "recipe-app"],
        &["check", "--json", "--project", "old"],
        &[
            "check",
            "--json",
            "--project",
            "recipe-app",
            "--name",
            "2026-11-17-b",
        ],
        &["check", "--json", "--name", "nothing"],
        &["check", "--json", "extra"],
    ] {
        same_check(&f, &f.root, args);
    }
}

#[test]
fn check_edge_tree_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = edge_tree();
    let (_, yaml) = same_check(&f, &f.root, &["check", "--json"]);
    assert!(yaml > 0, "YAML のエラーの文言の例外を確かめていない");
    same_check(&f, &f.root, &["check", "--json", "--project", "alpha"]);
}

/// ナレッジの詳細には principle を求めない（Go の TestKnowledgeDetailNeedsNoPrinciple）。
#[test]
fn check_knowledge_detail_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = recipe_tree();
    f.write(
        "knowledge/derive-state-from-input.md",
        "---\ndescription: 画面の状態を直すときに読む。状態を入力から導く\nprinciple: ../principles/prefer-structure-over-patches.md\nsources: []\n---\n\n# 状態は入力から導く\n\n手順は [詳細](derive-state-from-input-steps.md)。\n",
    );
    f.write(
        "knowledge/derive-state-from-input-steps.md",
        "---\ndescription: derive-state-from-input.md から指されたときだけ読む。導き方の手順\n---\n\n# 導き方の手順\n",
    );
    f.write(
        "knowledge/orphan.md",
        "---\ndescription: どこからも指されていない\n---\n\n# 指されていない\n",
    );
    f.write(
        "projects/recipe-app/materials/2026-11-10-no-source.md",
        "---\nknowledge: []\n---\n\n# ツリーに場所が無い出来事\n\n## 何が起きたか\nx\n\n## 一般化の候補\n- x\n",
    );
    same_check(&f, &f.root, &["check", "--json"]);
}

/// 手元の実際のツリーの複製でも比べる（DESK_DIFF_TREE にツリーのルートを渡したときだけ）。結果はリポジトリに入れない。
#[test]
fn real_tree_matches_go() {
    let Some(src) = std::env::var_os("DESK_DIFF_TREE") else {
        return;
    };
    if go_desk().is_none() {
        return;
    }
    let f = Fixture::copy_of(Path::new(&src));
    same_check(&f, &f.root, &["check", "--json"]);
    for args in [
        &["ls", "--archived", "--paths"][..],
        &[
            "ls",
            "--archived",
            "--json",
            "--fields",
            "status,parent,description",
        ],
        &[
            "ls",
            "task",
            "--where",
            "status~todo,inprogress",
            "--level",
            "0",
        ],
        &["ls", "knowledge", "--level", "0"],
        &["graph", "--all", "--format", "dot"],
    ] {
        same_in(&f.root, args);
    }
    let tree = Tree::load(&f.root).unwrap();
    for e in tree.entries(&[], None, false) {
        if e.info.kind == Some(desk_core::Kind::Project) {
            continue;
        }
        let p = e.info.path.as_str();
        for args in [
            &["read", p, "--level", "0"][..],
            &["read", p, "--outline"],
            &["read", p, "--level", "2"],
            &["graph", p, "--depth", "2"],
        ] {
            same_in(&f.root, args);
        }
    }
}
