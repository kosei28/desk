//! 書く側のテスト（Go の cmd_new_test・cmd_move_test・cmd_check_test の --fix のうち、出力の形によらないところを
//! core の API で見る）。出力の形は cli/tests で見る。

#![cfg(feature = "write")]

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::{Fixture, recipe_tree, task_doc};
use desk_core::Tree;
use desk_core::write::{self, Error, FixPlan, NewRequest};

fn read(f: &Fixture, rel: &str) -> String {
    fs::read_to_string(f.path(rel)).unwrap()
}

fn exists(f: &Fixture, rel: &str) -> bool {
    f.path(rel).exists()
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=desk", "-c", "user.email=desk@example.com"])
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}");
    String::from_utf8(o.stdout).unwrap()
}

fn load(f: &Fixture) -> Tree {
    Tree::load(&f.root).unwrap()
}

// init（Go の TestInit*）

#[test]
fn init_creates_tree() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap().join("notes");
    let res = write::init(&root).unwrap();
    assert!(!res.already && res.committed, "{res:?}");
    for rel in [
        ".desk",
        "README.md",
        ".gitignore",
        "projects/misc/README.md",
    ] {
        assert!(root.join(rel).exists(), "{rel} が無い");
    }
    for rel in ["principles", "knowledge", "archive"] {
        assert!(
            !root.join(rel).exists(),
            "空のディレクトリ {rel} を作っている"
        );
    }
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.starts_with("# notes\n"), "{readme}");
    assert!(
        readme.contains("projects/misc/inbox/"),
        "README に人が書く受け箱の場所が無い"
    );
    assert_eq!(
        git_out(&root, &["log", "--format=%s"]).trim(),
        "docs: desk を作る"
    );

    let again = write::init(&root).unwrap();
    assert!(again.already && again.created.is_empty(), "{again:?}");
}

#[test]
fn init_in_existing_repository() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join("README.md"), "# 前からある\n").unwrap();
    git_out(&root, &["init", "-q", "-b", "main"]);
    git_out(&root, &["add", "README.md"]);
    git_out(&root, &["commit", "-q", "-m", "first"]);
    let res = write::init(&root).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("README.md")).unwrap(),
        "# 前からある\n",
        "あるファイルを書き換えている"
    );
    assert!(!res.committed && root.join(".desk").exists(), "{res:?}");
    assert!(!res.written.contains(&"README.md".to_string()), "{res:?}");
    assert_eq!(git_out(&root, &["log", "--format=%s"]).trim(), "first");
}

#[test]
fn init_inside_tree_fails() {
    let f = Fixture::new();
    let inner = f.path("projects/misc/nested");
    let err = write::init(&inner).unwrap_err();
    assert!(
        err.to_string().contains("ツリーの中にツリーは作らない"),
        "{err}"
    );
    assert!(!inner.exists(), "止まったのにディレクトリを作っている");
    let file = f.path("README.md");
    assert!(write::init(&file).is_err(), "ファイルの場所に作れてしまう");
}

// new（Go の TestNewProjectAndTasks・TestNewRefusesNameUsedInArchive）

fn req<'a>(kind: &'a str, slug: &'a str) -> NewRequest<'a> {
    NewRequest {
        kind,
        slug,
        today: "2026-11-14",
        ..Default::default()
    }
}

fn new(f: &Fixture, r: NewRequest<'_>) -> Result<String, Error> {
    let t = load(f);
    let plan = t.plan_new(&r, &f.root)?;
    plan.apply(&t)?;
    Ok(plan.path)
}

#[test]
fn new_project_and_tasks() {
    let f = Fixture::new();
    assert_eq!(
        new(&f, req("project", "recipe-app")).unwrap(),
        "projects/recipe-app/README.md"
    );
    assert_eq!(read(&f, "projects/recipe-app/README.md"), "# recipe-app\n");
    assert!(
        matches!(new(&f, req("project", "recipe-app")), Err(Error::Failed(_))),
        "同じプロジェクトを 2 回作れてしまう"
    );

    let parent = new(
        &f,
        NewRequest {
            project: Some("recipe-app"),
            title: Some("献立プランナー"),
            ..req("task", "2026-09-07-meal-planner")
        },
    )
    .unwrap();
    assert_eq!(
        parent,
        "projects/recipe-app/tasks/2026-09-07-meal-planner.md"
    );
    let c = read(&f, &parent);
    for want in [
        "status: todo",
        "needs_human: false",
        "# 献立プランナー",
        "## 目的",
        "## いま",
        "## 成果物",
        "## 現在地",
        "## 決まり",
        "## 手順",
    ] {
        assert!(c.contains(want), "タスクに {want} が無い:\n{c}");
    }

    let child = new(
        &f,
        NewRequest {
            parent: Some("2026-09-07-meal-planner"),
            needs_human: true,
            ..req("task", "2026-11-10-decide-paper-size")
        },
    )
    .unwrap();
    let c = read(&f, &child);
    assert!(
        c.contains("parent: 2026-09-07-meal-planner.md")
            && c.contains("needs_human: true")
            && !c.contains("## 現在地"),
        "人の子タスクの形が違う:\n{c}"
    );

    // 日付の無い slug は今日の日付で作る
    let today = new(
        &f,
        NewRequest {
            project: Some("recipe-app"),
            ..req("task", "shopping-list")
        },
    )
    .unwrap();
    assert_eq!(
        today,
        "projects/recipe-app/tasks/2026-11-14-shopping-list.md"
    );

    let art = new(
        &f,
        NewRequest {
            parent: Some("2026-09-07-meal-planner"),
            ..req("artifact", "print-layout")
        },
    )
    .unwrap();
    assert!(
        read(&f, &art).contains("tasks: [../tasks/2026-09-07-meal-planner.md]"),
        "artifact の tasks が親を指さない"
    );

    assert_eq!(
        new(&f, req("knowledge", "reproduce-before-fixing")).unwrap(),
        "knowledge/reproduce-before-fixing.md",
        "プロジェクトを付けないナレッジはルートに置く"
    );

    let err = new(
        &f,
        NewRequest {
            project: Some("nothing"),
            ..req("task", "x")
        },
    )
    .unwrap_err();
    assert!(
        err.is_usage() && err.to_string().contains("プロジェクト nothing が無い"),
        "{err}"
    );
    let err = new(
        &f,
        NewRequest {
            project: Some("recipe-app"),
            ..req("task", "Bad_Slug")
        },
    )
    .unwrap_err();
    assert!(err.is_usage(), "決まりの外の slug で作れてしまう: {err}");
}

#[test]
fn new_refuses_name_used_in_archive() {
    let f = recipe_tree();
    f.write(
        "projects/recipe-app/archive/tasks/2026-09-07-favorites.md",
        &task_doc("done", "", "お気に入り"),
    );
    let err = new(
        &f,
        NewRequest {
            project: Some("recipe-app"),
            ..req("task", "2026-09-07-favorites")
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("既にある"), "{err}");
    assert!(!exists(
        &f,
        "projects/recipe-app/tasks/2026-09-07-favorites.md"
    ));
}

#[test]
fn new_apply_does_not_overwrite() {
    let f = recipe_tree();
    let t = load(&f);
    let plan = t
        .plan_new(
            &NewRequest {
                project: Some("recipe-app"),
                ..req("task", "2026-11-14-x")
            },
            &f.root,
        )
        .unwrap();
    // 計画の後に同じ名前ができた（同時に作った）
    f.write(&plan.path, "先に書いた\n");
    let err = plan.apply(&t).unwrap_err();
    assert!(err.to_string().contains("既にある"), "{err}");
    assert_eq!(read(&f, &plan.path), "先に書いた\n");
}

// archive・mv（Go の cmd_move_test）

#[test]
fn archive_moves_and_rewrites_links() {
    let f = recipe_tree();
    f.commit_all("2026-11-01");
    let task = "projects/recipe-app/tasks/2026-11-09-printable-menu.md";
    let args = [
        task,
        "projects/recipe-app/artifacts/print-layout.md",
        "projects/recipe-app/artifacts/assets/print-sample.png",
    ];

    // 計画だけでは動かない
    let t = load(&f);
    let a = t.plan_archive(&args, None, &f.root);
    assert!(a.already.is_empty());
    let plan = a.plan.unwrap().unwrap();
    assert_eq!(plan.moves.len(), 3);
    assert!(exists(&f, task), "計画で動いた");

    let res = plan.apply(&t).unwrap();
    let moved = "projects/recipe-app/archive/tasks/2026-11-09-printable-menu.md";
    assert!(!exists(&f, task) && exists(&f, moved));
    assert!(exists(
        &f,
        "projects/recipe-app/archive/artifacts/assets/print-sample.png"
    ));
    let c = read(&f, moved);
    assert!(
        c.contains("status: done"),
        "status が done にならない:\n{c}"
    );
    assert!(
        c.contains("parent: ../../tasks/2026-09-07-meal-planner.md"),
        "動かしたファイルの中のリンクを付け替えていない:\n{c}"
    );
    let a = read(&f, "projects/recipe-app/archive/artifacts/print-layout.md");
    assert!(
        a.contains("tasks: [../tasks/2026-11-09-printable-menu.md]")
            && a.contains("![](assets/print-sample.png)")
            && a.contains("(../../tasks/2026-09-07-meal-planner.md)"),
        "一緒に動いたもの同士・外へのリンク:\n{a}"
    );
    let w = read(&f, "projects/recipe-app/wiki/meal-planner/overview.md");
    assert!(
        w.contains("(../../archive/artifacts/print-layout.md)"),
        "外から指すリンクを付け替えていない:\n{w}"
    );
    assert!(
        res.rewritten
            .contains(&"projects/recipe-app/wiki/meal-planner/overview.md".to_string()),
        "{res:?}"
    );
    assert!(
        !exists(&f, "projects/recipe-app/artifacts"),
        "空になったディレクトリが残る"
    );
    let status = git_out(&f.root, &["status", "--porcelain"]);
    assert!(
        status.contains(&format!("{task} -> {moved}")),
        "git mv で動かしていない:\n{status}"
    );

    let t = load(&f);
    let again = t.plan_archive(&[moved], None, &f.root);
    assert_eq!(again.already, vec![moved.to_string()]);
    assert!(matches!(again.plan, Ok(None)), "{again:?}");
    let wiki = t.plan_archive(
        &["projects/recipe-app/wiki/meal-planner/overview.md"],
        None,
        &f.root,
    );
    assert!(wiki.plan.is_err(), "wiki を archive に移せてしまう");
}

#[test]
fn archive_project() {
    let f = recipe_tree();
    f.write(
        "projects/misc/artifacts/notes.md",
        "---\ntasks: []\n---\n\n[概要](../../recipe-app/wiki/meal-planner/overview.md)\n",
    );
    let t = load(&f);
    t.plan_archive_project("recipe-app")
        .unwrap()
        .apply(&t)
        .unwrap();
    assert!(
        !exists(&f, "projects/recipe-app")
            && exists(
                &f,
                "archive/projects/recipe-app/wiki/meal-planner/overview.md"
            ),
        "プロジェクトが動いていない"
    );
    let c = read(
        &f,
        "archive/projects/recipe-app/knowledge/api-error-format.md",
    );
    assert!(
        c.contains("principle: ../../../../principles/prefer-structure-over-patches.md"),
        "中から外へのリンク:\n{c}"
    );
    let c = read(&f, "archive/projects/recipe-app/artifacts/print-layout.md");
    assert!(
        c.contains("tasks: [../tasks/2026-11-09-printable-menu.md]"),
        "中同士のリンクが変わった:\n{c}"
    );
    let c = read(&f, "projects/misc/artifacts/notes.md");
    assert!(
        c.contains("(../../../archive/projects/recipe-app/wiki/meal-planner/overview.md)"),
        "外から指すリンク:\n{c}"
    );
}

#[test]
fn mv_rename_and_promote() {
    let f = recipe_tree();
    let mv = |srcs: &[&str], dest: &str| {
        let t = load(&f);
        t.plan_mv(srcs, dest, &f.root).and_then(|p| p.apply(&t))
    };
    mv(
        &["projects/recipe-app/artifacts/print-layout.md"],
        "projects/recipe-app/artifacts/print-columns.md",
    )
    .unwrap();
    let c = read(&f, "projects/recipe-app/wiki/meal-planner/overview.md");
    assert!(
        c.contains("(../../artifacts/print-columns.md)"),
        "名前の変更でリンクを付け替えていない:\n{c}"
    );

    mv(
        &["projects/recipe-app/knowledge/api-error-format.md"],
        "knowledge/",
    )
    .unwrap();
    let c = read(&f, "knowledge/api-error-format.md");
    assert!(
        c.contains("principle: ../principles/prefer-structure-over-patches.md"),
        "ルートに移したナレッジのリンク:\n{c}"
    );

    let err = mv(
        &["projects/recipe-app/artifacts/print-columns.md"],
        "projects/recipe-app/drafts/",
    )
    .unwrap_err();
    assert!(err.to_string().contains("決まりの外"), "{err}");
}

// check --fix（Go の TestCheckFix・TestCheckFixLeavesManualFindings）

fn fix(f: &Fixture) -> Vec<String> {
    let t = load(f);
    FixPlan::new(&t.check()).apply(&t).unwrap()
}

#[test]
fn check_fix() {
    let f = recipe_tree();
    f.commit_all("2026-11-01");
    let p = "projects/recipe-app/";
    f.write(
        &format!("{p}tasks/2026-11-02-no-fm.md"),
        "# 人が書いたタスク\n\n## 目的\nx\n\n## いま\nx\n\n## 成果物\nx\n\n## 現在地\nx\n\n## 決まり\nx\n\n## 手順\nx\n",
    );
    f.write(
        &format!("{p}tasks/2026-11-04-closed.md"),
        &(task_doc("done", "", "閉じた") + "\n[設計](../artifacts/print-layout.md)\n"),
    );
    f.write(
        &format!("{p}artifacts/moved-by-hand.md"),
        "---\ntasks: []\n---\n\n[概要](overview.md)\n",
    );
    f.write(
        &format!("{p}artifacts/no-tasks.md"),
        "---\ntitle: x\n---\n# tasks が無い\n",
    );

    let changed = fix(&f);
    let c = read(&f, &format!("{p}tasks/2026-11-02-no-fm.md"));
    assert!(
        c.starts_with("---\nstatus:\nneeds_human:\n---\n\n# 人が書いたタスク"),
        "frontmatter を足していない（値は埋めない）:\n{c}"
    );
    // status と needs_human の値は手で書くので残る
    let left: Vec<(String, String)> = load(&f)
        .check()
        .into_iter()
        .map(|x| (x.path, x.problem))
        .collect();
    let no_fm = format!("{p}tasks/2026-11-02-no-fm.md");
    assert_eq!(
        left,
        vec![
            (no_fm.clone(), "status が空".to_string()),
            (no_fm, "needs_human が空".to_string()),
        ],
        "--fix の後に残るのが status・needs_human の空だけでない"
    );
    let moved = format!("{p}archive/tasks/2026-11-04-closed.md");
    assert!(exists(&f, &moved) && !exists(&f, &format!("{p}tasks/2026-11-04-closed.md")));
    let c = read(&f, &moved);
    assert!(
        c.contains("(../../artifacts/print-layout.md)"),
        "移したタスクのリンク:\n{c}"
    );
    let c = read(&f, &format!("{p}artifacts/moved-by-hand.md"));
    assert!(
        c.contains("(../wiki/meal-planner/overview.md)"),
        "切れたリンクを付け替えていない:\n{c}"
    );
    let c = read(&f, &format!("{p}artifacts/no-tasks.md"));
    assert!(c.contains("tasks: []"), "必須キーを足していない:\n{c}");
    assert!(changed.contains(&moved), "{changed:?}");

    // もう直すものが無い
    assert!(fix(&f).is_empty());
}

#[test]
fn check_fix_leaves_manual_findings() {
    let f = recipe_tree();
    f.write("projects/recipe-app/knowledge/empty.md", "# 中身だけ\n");
    fix(&f);
    assert!(
        !load(&f).check().is_empty(),
        "手で直すもの（description・principle が空）が残らない"
    );
    let c = read(&f, "projects/recipe-app/knowledge/empty.md");
    assert!(
        c.starts_with("---\ndescription:\nprinciple:\n---\n"),
        "空のキーを足していない:\n{c}"
    );
}
