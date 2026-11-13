//! 引く・読む・確かめるのテスト（何に当たるか・何が挙がるかを core の API で見る）。出力の形は cli/tests で見る。

mod common;

use common::{Fixture, check_tree, recipe_tree, task_doc};
use desk_core::Tree;
use desk_core::check::{Category, Finding};
use desk_core::graph::Direction;
use desk_core::query::{Query, Where};
use desk_core::rules::describe;
use desk_core::tree::ResolveError;
use desk_core::{Kind, find_root};

fn paths(t: &Tree, q: Query) -> Vec<String> {
    t.ls(&q).into_iter().map(|e| e.info.path).collect()
}

fn wheres(ws: &[&str]) -> Vec<Where> {
    ws.iter().map(|w| Where::parse(w).unwrap()).collect()
}

#[test]
fn ls_filters() {
    let f = recipe_tree();
    f.write(
        "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md",
        "---\nstatus: todo\nneeds_human: true\nparent: 2026-11-09-printable-menu.md\n---\n\n# 用紙の大きさを決める\n\n## 目的\n選ぶ。\n\n## 成果物\n- なし\n",
    );
    f.write(
        "projects/recipe-app/archive/tasks/2026-09-07-favorites.md",
        &task_doc("done", "", "お気に入り"),
    );
    let t = Tree::load(&f.root).unwrap();
    let task = |ws: &[&str], archived: bool| {
        paths(
            &t,
            Query {
                kinds: vec![Kind::Task],
                wheres: wheres(ws),
                archived,
                ..Query::default()
            },
        )
    };

    assert_eq!(
        task(&["needs_human=true"], false),
        ["projects/recipe-app/tasks/2026-11-10-decide-paper-size.md"]
    );
    assert!(task(&[], false).iter().all(|p| !p.contains("archive/")));
    assert!(
        task(&[], true)
            .iter()
            .any(|p| p.ends_with("archive/tasks/2026-09-07-favorites.md"))
    );
    assert_eq!(
        task(&["parent=2026-09-07-meal-planner"], false),
        ["projects/recipe-app/tasks/2026-11-09-printable-menu.md"]
    );
    let of = |kind: Kind, ws: &[&str], project: Option<&str>| {
        paths(
            &t,
            Query {
                kinds: vec![kind],
                wheres: wheres(ws),
                project: project.map(str::to_string),
                ..Query::default()
            },
        )
    };
    assert_eq!(
        of(Kind::Artifact, &["tasks~printable-menu"], None),
        ["projects/recipe-app/artifacts/print-layout.md"]
    );
    assert_eq!(
        of(Kind::Knowledge, &["sources=[]"], None),
        ["projects/recipe-app/knowledge/api-error-format.md"]
    );
    assert_eq!(
        of(Kind::Principle, &[], Some("recipe-app")),
        ["principles/prefer-structure-over-patches.md"]
    );
    assert_eq!(
        of(Kind::Project, &[], None),
        ["projects/misc", "projects/recipe-app"]
    );
    assert_eq!(
        of(Kind::Wiki, &[], None),
        ["projects/recipe-app/wiki/meal-planner/overview.md"]
    );
    assert_eq!(
        paths(
            &t,
            Query {
                name: Some("recipe-app".into()),
                ..Query::default()
            }
        ),
        ["projects/recipe-app"]
    );
}

#[test]
fn where_errors() {
    assert_eq!(
        Where::parse("=x").unwrap_err().to_string(),
        r#"--where "=x": キーが無い（例: status=inprogress）"#
    );
    assert_eq!(
        Where::parse("a?b").unwrap_err().to_string(),
        r#"--where "a?b": 演算子は = != ~ < >"#
    );
}

#[test]
fn graph_walks() {
    let f = recipe_tree();
    f.write(
        "projects/recipe-app/tasks/2026-11-11-print-preview.md",
        &task_doc(
            "todo",
            "parent: 2026-11-09-printable-menu.md",
            "印刷のプレビューを出す",
        ),
    );
    let t = Tree::load(&f.root).unwrap();
    let w = t.walk(
        "projects/recipe-app/tasks/2026-09-07-meal-planner.md",
        &["parent".into()],
        &[Direction::In],
        None,
    );
    assert_eq!(
        w.order,
        [
            "projects/recipe-app/tasks/2026-11-09-printable-menu.md",
            "projects/recipe-app/tasks/2026-11-11-print-preview.md"
        ]
    );
    assert_eq!(w.steps[1].depth, 2);

    let w = t.walk(
        "projects/recipe-app/tasks/2026-11-09-printable-menu.md",
        &[],
        &[Direction::In],
        Some(1),
    );
    assert!(
        w.order
            .contains(&"projects/recipe-app/artifacts/print-layout.md".to_string())
    );
    assert!(
        w.order
            .contains(&"projects/recipe-app/tasks/2026-11-11-print-preview.md".to_string())
    );
    // 1 つの点の辺はキー、そして指している側のパスの順
    let keys: Vec<(&str, &str)> = w
        .steps
        .iter()
        .map(|s| (s.key.as_str(), s.node.as_str()))
        .collect();
    assert!(keys.is_sorted() && keys.len() == 2, "{keys:?}");

    let w = t.walk(
        "projects/recipe-app/artifacts/print-layout.md",
        &[],
        &[Direction::Out],
        Some(1),
    );
    let got: Vec<(&str, &str)> = w
        .steps
        .iter()
        .map(|s| (s.key.as_str(), s.node.as_str()))
        .collect();
    assert!(got.contains(&(
        "tasks",
        "projects/recipe-app/tasks/2026-11-09-printable-menu.md"
    )));
    assert!(got.contains(&(
        "body",
        "projects/recipe-app/artifacts/assets/print-sample.png"
    )));

    // 両方の向き。もう出たものは（既出）として 1 歩だけ残り、その先はたどらない
    let w = t.walk(
        "projects/recipe-app/tasks/2026-11-09-printable-menu.md",
        &[],
        &[Direction::Out, Direction::In],
        None,
    );
    assert!(w.steps.iter().any(|s| s.seen));
    let mut uniq = w.order.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), w.order.len());
}

#[test]
fn read_levels() {
    let f = recipe_tree();
    let t = Tree::load(&f.root).unwrap();
    let task = t
        .get("projects/recipe-app/tasks/2026-11-09-printable-menu.md")
        .unwrap();
    assert_eq!(task.status(), "inprogress");
    assert_eq!(task.now(), "いま。");

    let s = task.section("目的").unwrap();
    assert_eq!(s, ["## 目的", "目的。"]);
    assert!(task.section("無い節").is_none());

    let outline: Vec<(usize, &str, usize)> = task
        .headings
        .iter()
        .map(|h| (h.level, h.text.as_str(), h.lines()))
        .collect();
    assert_eq!(
        (outline[0].0, outline[0].1),
        (1, "献立を印刷できるようにする")
    );
    assert!(outline.contains(&(2, "目的", 2)));
    assert!(outline.iter().any(|h| h.1 == "ログ"));
    assert_eq!(task.fm_lines(), Some(5));

    let wiki = t
        .get("projects/recipe-app/wiki/meal-planner/overview.md")
        .unwrap();
    let linked: Vec<&str> = t
        .linked_docs(wiki)
        .iter()
        .map(|d| d.info.path.as_str())
        .collect();
    assert_eq!(linked, ["projects/recipe-app/artifacts/print-layout.md"]);
}

#[test]
fn resolve_ambiguous_name() {
    let f = recipe_tree();
    f.write(
        "projects/misc/artifacts/print-layout.md",
        "---\ntasks: []\n---\n# 別の\n",
    );
    let t = Tree::load(&f.root).unwrap();
    let err = t.resolve("print-layout", &f.root).unwrap_err();
    assert!(matches!(err, ResolveError::Ambiguous { .. }));
    assert!(err.to_string().contains("複数ある"));
    assert_eq!(
        t.resolve("print-layout", &f.path("projects/recipe-app"))
            .unwrap(),
        "projects/recipe-app/artifacts/print-layout.md"
    );
    assert_eq!(
        t.resolve(
            "tasks/2026-09-07-meal-planner.md",
            &f.path("projects/recipe-app")
        )
        .unwrap(),
        "projects/recipe-app/tasks/2026-09-07-meal-planner.md"
    );
    assert!(matches!(
        t.resolve("/tmp", &f.root),
        Err(ResolveError::Outside { .. })
    ));
    assert_eq!(find_root(&f.path("projects/recipe-app")).unwrap(), f.root);
}

fn has(fs: &[Finding], path: &str, problem: &str) -> bool {
    fs.iter()
        .any(|f| f.path == path && f.problem.contains(problem))
}

#[test]
fn check_clean_tree() {
    let f = recipe_tree();
    assert_eq!(Tree::load(&f.root).unwrap().check(), []);
}

#[test]
fn check_findings() {
    let f = check_tree();
    let t = Tree::load(&f.root).unwrap();
    let fs = t.check();
    let p = "projects/recipe-app/";
    for (path, problem) in [
        ("notes/".to_string(), "決まった名前だけ"),
        (format!("{p}drafts/"), "プロジェクトに置けないディレクトリ"),
        (format!("{p}tasks/no-date.md"), "日付が無い"),
        (
            format!("{p}tasks/2026-11-01-bad-status.md"),
            r#"status の値 "doing""#,
        ),
        (
            format!("{p}tasks/2026-11-02-no-fm.md"),
            "frontmatter が無い",
        ),
        (
            format!("{p}tasks/2026-11-03-short.md"),
            "必須の節「いま」が無い",
        ),
        (
            format!("{p}tasks/2026-11-03-short.md"),
            "ログの節が最後にない",
        ),
        (format!("{p}tasks/2026-11-04-closed.md"), "archive/ の外"),
        (
            format!("{p}archive/tasks/2026-11-05-reopened.md"),
            "archive/ の中",
        ),
        (
            format!("{p}tasks/2026-11-06-started.md"),
            "depends_on の projects/recipe-app/tasks/2026-11-07-dep.md が todo なのに inprogress",
        ),
        (
            format!("{p}tasks/2026-11-10-planned.md"),
            "todo なのに depends_on の projects/recipe-app/tasks/2026-11-10-someday.md が backlog",
        ),
        (format!("{p}tasks/2026-11-11-on-dropped.md"), "が dropped"),
        (
            format!("{p}tasks/2026-11-12-started-child.md"),
            "が todo なのに inprogress",
        ),
        (
            format!("{p}tasks/2026-11-13-todo-child.md"),
            "が backlog なのに todo",
        ),
        (
            format!("{p}tasks/2026-11-14-left-child.md"),
            "が done なのに todo",
        ),
        (
            format!("{p}tasks/2026-11-15-self.md"),
            "depends_on が自分自身を指している",
        ),
        (
            format!("{p}tasks/2026-11-16-a.md"),
            "depends_on が循環している",
        ),
        (
            format!("{p}tasks/2026-11-16-b.md"),
            "depends_on が循環している",
        ),
        (format!("{p}tasks/2026-11-17-a.md"), "parent が循環している"),
        (format!("{p}tasks/2026-11-17-b.md"), "parent が循環している"),
        (
            format!("{p}tasks/2026-11-18-no-needs-human.md"),
            "needs_human が無い",
        ),
        (
            format!("{p}tasks/2026-11-19-old-status.md"),
            r#"status の値 "active""#,
        ),
        (
            format!("{p}tasks/2026-11-20-bad-needs-human.md"),
            r#"needs_human の値 "human""#,
        ),
        (format!("{p}artifacts/links.md"), "gone.md の先が無い"),
        (
            format!("{p}artifacts/links.md"),
            "/tmp/x.md が相対パスでない",
        ),
        (format!("{p}artifacts/links.md"), "[[名前]] が名前だけ"),
        (format!("{p}artifacts/links.md"), "名前が重複"),
        (
            format!("{p}artifacts/assets/unused.png"),
            "どこからも指されていない",
        ),
        (format!("{p}knowledge/no-principle.md"), "principle が無い"),
    ] {
        assert!(has(&fs, &path, problem), "挙がらない: {path} {problem}");
    }
    assert!(
        !fs.iter()
            .any(|f| f.path == "notes/todo.md" || f.path == format!("{p}drafts/x.md")),
        "決まりの外のディレクトリの中を 1 つずつ挙げている"
    );

    // 値の行（1 始まり）: 値が次の行にあればその行、空ならキーの行、ブロックの配列は最初の要素の行
    let line = |path: &str, problem: &str| {
        fs.iter()
            .find(|f| f.path == format!("{p}{path}") && f.problem.contains(problem))
            .unwrap_or_else(|| panic!("挙がらない: {path} {problem}"))
            .line
    };
    assert_eq!(line("tasks/2026-11-01-bad-status.md", "status の値"), 2);
    // 値は Rust の Debug で引用する（見えない文字は \u{…} にエスケープ）
    assert_eq!(line("tasks/2026-11-21-values.md", r#""to\u{a0}do""#), 3);
    assert_eq!(line("tasks/2026-11-21-values.md", "needs_human が空"), 4);
    assert_eq!(line("knowledge/bad-values.md", "description が空"), 2);
    assert_eq!(line("knowledge/bad-values.md", "verified の値"), 6);
    assert_eq!(line("knowledge/empty-list.md", "principle が空"), 3);
    let fixable = |path: &str, problem: &str| {
        fs.iter()
            .find(|f| f.path == format!("{p}{path}") && f.problem.contains(problem))
            .unwrap()
            .fixable
    };
    assert!(fixable("tasks/2026-11-02-no-fm.md", "frontmatter が無い"));
    assert!(fixable("tasks/2026-11-04-closed.md", "archive/ の外"));
    assert!(!fixable("tasks/2026-11-22-closed.md", "archive/ の外"));
    assert!(fixable(
        "artifacts/links.md",
        "../overview.md#節 の先が無い"
    ));
    assert!(!fixable("artifacts/links.md", "gone.md の先が無い"));
    assert!(!fixable("tasks/2026-11-01-bad-status.md", "status の値"));

    // 子が全部終わっていても、親が inprogress なのはずれではない
    let parent = format!("{p}tasks/2026-11-08-parent.md");
    let child = format!("{p}archive/tasks/2026-11-08-child.md");
    assert!(
        !fs.iter().any(|f| f.concerns(&parent) || f.concerns(&child)),
        "子が全部終わった inprogress の親を挙げている"
    );

    // --name は関係するファイルの分も出す
    let parent = format!("{p}tasks/2026-11-12-todo-parent.md");
    let child = format!("{p}tasks/2026-11-12-started-child.md");
    let named: Vec<&Finding> = fs.iter().filter(|f| f.concerns(&parent)).collect();
    assert!(named.iter().any(|f| f.path == child
        && f.category == Category::Drift
        && f.problem.contains("が todo なのに inprogress")));
    assert!(named.iter().all(|f| f.path == parent || f.path == child));

    // --project はそのプロジェクトだけ
    assert!(!fs.iter().any(|f| f.in_project("misc")));
    assert!(fs.iter().any(|f| f.in_project("old")));
}

#[test]
fn check_needless_depends() {
    let f = Fixture::new();
    f.write(
        "projects/deps/README.md",
        "# deps\n\n依存の形を見るプロジェクト。\n",
    );
    let path = |name: &str| format!("projects/deps/tasks/2026-11-01-{name}.md");
    let task = |name: &str, parent: &str, deps: &[&str]| {
        let mut fm = Vec::new();
        if !parent.is_empty() {
            fm.push(format!("parent: 2026-11-01-{parent}.md"));
        }
        if !deps.is_empty() {
            let deps: Vec<String> = deps.iter().map(|d| format!("2026-11-01-{d}.md")).collect();
            fm.push(format!("depends_on: [{}]", deps.join(", ")));
        }
        f.write(&path(name), &task_doc("todo", &fm.join("\n"), name));
    };
    // 祖先と子孫の間: 孫も、子孫から祖先への向きも挙げる。兄弟の間は挙げない
    task("a-parent", "", &["a-grandchild"]);
    task("a-child", "a-parent", &[]);
    task("a-grandchild", "a-child", &[]);
    task("a-child2", "a-parent", &["a-child"]);
    task("g-parent", "", &[]);
    task("g-child", "g-parent", &[]);
    task("g-grandchild", "g-child", &["g-parent"]);
    // 依存の依存
    task("b-x", "", &["b-y", "b-z"]);
    task("b-y", "", &["b-z"]);
    task("b-z", "", &[]);
    // 祖先の依存を子孫で書き直す。特定の子だけ外の別のタスクに依存するのは挙げない
    task("c-parent", "", &["c-ext"]);
    task("c-child", "c-parent", &["c-ext"]);
    task("c-child2", "c-parent", &["c-other"]);
    task("c-ext", "", &[]);
    task("c-other", "", &[]);
    // 依存先の子孫。子孫だけに依存するのは挙げない
    task("d-x", "", &["d-target", "d-target-child"]);
    task("d-y", "", &["d-target-child"]);
    task("d-target", "", &[]);
    task("d-target-child", "d-target", &[]);
    // 引き継ぎ・推移・依存先の子孫を合わせて導ける
    task("e-parent", "", &["e-mid"]);
    task("e-child", "e-parent", &["e-end-child"]);
    task("e-mid", "", &["e-end"]);
    task("e-end", "", &[]);
    task("e-end-child", "e-end", &[]);
    // 親子の間の依存からは導かない（兄弟の間の依存は残す）
    task("f-parent", "", &["f-child"]);
    task("f-child", "f-parent", &[]);
    task("f-child2", "f-parent", &["f-child"]);

    let fs = Tree::load(&f.root).unwrap().check();
    let kin = |from: &str, to: &str, kin: &str| {
        (
            path(from),
            format!(
                "depends_on の {} が{kin}のタスク（親子の間には依存を張らない）",
                path(to)
            ),
        )
    };
    let derived = |from: &str, to: &str| {
        (
            path(from),
            format!("depends_on の {} はほかの依存から導ける", path(to)),
        )
    };
    let mut want = vec![
        kin("a-parent", "a-grandchild", "子孫"),
        derived("b-x", "b-z"),
        derived("c-child", "c-ext"),
        derived("d-x", "d-target-child"),
        derived("e-child", "e-end-child"),
        kin("f-parent", "f-child", "子孫"),
        kin("g-grandchild", "g-parent", "祖先"),
    ];
    want.sort();
    let got: Vec<(String, String)> = fs
        .iter()
        .map(|f| (f.path.clone(), f.problem.clone()))
        .collect();
    assert_eq!(got, want);
    assert!(
        fs.iter()
            .all(|f| f.category == Category::Drift && f.fix == "depends_on から外す" && !f.fixable)
    );

    // --name は依存先の分も出す
    assert!(
        fs.iter()
            .any(|f| f.path == path("a-parent") && f.concerns(&path("a-grandchild")))
    );
}

#[test]
fn check_rules() {
    let out = describe();
    for want in [
        "正本はスキル desk",
        "task —",
        "status: 必須",
        "backlog | todo | inprogress | done | dropped",
        "YYYY-MM-DD-<slug>.md",
        "projects/<p>/tasks/",
    ] {
        assert!(out.contains(want), "--rules に {want:?} が無い:\n{out}");
    }
}

#[test]
fn knowledge_with_several_principles() {
    let f = recipe_tree();
    f.write(
        "principles/single-source-of-truth.md",
        "---\ndescription: 1 つの事実は 1 か所に置く\n---\n\n# 事実は 1 か所\n",
    );
    f.write(
        "knowledge/derive-state-from-input.md",
        "---\ndescription: 画面の状態を直すときに読む。状態を入力から導く\nprinciple: [../principles/prefer-structure-over-patches.md, ../principles/single-source-of-truth.md]\nsources: []\n---\n\n# 状態は入力から導く\n",
    );
    let t = Tree::load(&f.root).unwrap();
    assert_eq!(t.check(), []);
    for p in [
        "principles/prefer-structure-over-patches.md",
        "principles/single-source-of-truth.md",
    ] {
        let w = t.walk(p, &["principle".into()], &[Direction::In], Some(1));
        assert!(
            w.order
                .contains(&"knowledge/derive-state-from-input.md".to_string()),
            "{p} から指しているナレッジが出ない"
        );
    }
    assert_eq!(
        paths(
            &t,
            Query {
                kinds: vec![Kind::Knowledge],
                wheres: wheres(&["principle=single-source-of-truth"]),
                ..Query::default()
            }
        ),
        ["knowledge/derive-state-from-input.md"]
    );
}

#[test]
fn knowledge_detail_needs_no_principle() {
    let f: Fixture = recipe_tree();
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
    let fs = Tree::load(&f.root).unwrap().check();
    assert!(!fs.is_empty());
    assert!(fs.iter().all(|f| f.path == "knowledge/orphan.md"), "{fs:?}");
    assert!(has(&fs, "knowledge/orphan.md", "principle が無い"));
}
