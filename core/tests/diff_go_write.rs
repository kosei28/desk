//! 差分テスト（R3）: 書く側。同じツリーの写しを 2 つ作り、片方に Go の CLI、もう片方に core を呼んで同じ形に整えたもの
//! （tests/common/cli.rs）を当てて、標準出力・標準エラー・終了コードと、結果のツリー（ファイルの中身、空のディレクトリ、
//! git の index の状態）を比べる。出力の中のルートの絶対パスは、写しごとに違うので `<root>` に置き換えて比べる。
//! 当てるのはいつも一時ディレクトリの写しで、手元のツリーは書き換えない。
//! Go の CLI を消す切り替え（R5）で、Go と一緒に消す。
//!
//! - `init`: 新しいディレクトリ、作業ディレクトリ、git のあるディレクトリ、もうルート、ツリーの中
//! - `new`: 全部の種類、`--project`・`--parent`・`--needs-human`・`--title`、日付付きの slug、止まるとき
//! - `archive`: `--dry-run`、`--status`、もう archive/ にあるもの、`--project`、止まるとき
//! - `mv`: 名前の変更、ディレクトリへ、ディレクトリごと、作業ディレクトリからの先、止まるとき
//! - `check --fix`: `--json` と、全部直って表の要らないとき。`--name`・`--project` で絞ったとき
//!
//! 使い方の文言と表（`check` の既定の出力）は CLI の段（R4）で比べる。
//! Go が要る。`go` が無ければ失敗する（Go を入れずに飛ばすなら DESK_SKIP_GO_DIFF=1）。

#![cfg(feature = "write")]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use common::cli::{self, Out};
use common::go::{fix_go_fm_lines, go_desk, go_run};
use common::{Fixture, check_tree, recipe_tree, task_doc};

/// ツリーの中身: ルートからのパス → 中身（ディレクトリは None）。.git の中は見ない。
fn snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(dir: &Path, rel: &str, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            if name == ".git" {
                continue;
            }
            let r = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if e.file_type().unwrap().is_dir() {
                out.insert(format!("{r}/"), None);
                walk(&e.path(), &r, out);
            } else {
                out.insert(r, Some(fs::read(e.path()).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        walk(root, "", &mut out);
    }
    out
}

/// git の index と作業ツリーの状態（git でなければ空）。
fn git_state(root: &Path) -> String {
    if !root.join(".git").exists() {
        return String::new();
    }
    let git = |args: &[&str]| {
        let o = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        String::from_utf8(o.stdout).unwrap()
    };
    git(&["status", "--porcelain", "-uall"]) + &git(&["log", "--format=%s"])
}

/// 出力の中のルート（写しごとに違う）を `<root>` にする。
fn norm(mut o: Out, base: &Path) -> Out {
    let b = base.display().to_string();
    o.stdout = o.stdout.replace(&b, "<base>");
    o.stderr = o.stderr.replace(&b, "<base>");
    o
}

/// 写しの結果が同じか見る。
fn same_trees(a: &Path, b: &Path, what: &str) {
    let (sa, sb) = (snapshot(a), snapshot(b));
    if sa != sb {
        let keys: Vec<&String> = sa
            .keys()
            .chain(sb.keys())
            .filter(|k| sa.get(*k) != sb.get(*k))
            .collect();
        for k in &keys {
            let show = |v: Option<&Option<Vec<u8>>>| match v {
                None => "（無い）".to_string(),
                Some(None) => "（ディレクトリ）".to_string(),
                Some(Some(c)) => String::from_utf8_lossy(c).into_owned(),
            };
            eprintln!(
                "--- {k}\nGo:\n{}\nRust:\n{}",
                show(sa.get(*k)),
                show(sb.get(*k))
            );
        }
        panic!("{what} の後のツリーが Go と違う: {keys:?}");
    }
    assert_eq!(git_state(b), git_state(a), "{what} の後の git が Go と違う");
}

/// f の写し 2 つに Go と core を当てる。返すのは（Go の写し、Go の結果、Rust の写し、Rust の結果）。
fn run_both(f: &Fixture, cwd: &str, args: &[&str]) -> (Fixture, Out, Fixture, Out) {
    let (a, b) = (f.duplicate(), f.duplicate());
    let base = |x: &Fixture| x.root.parent().unwrap().to_path_buf();
    let want = norm(go_run(&a.root.join(cwd), args), &base(&a));
    let got = norm(cli::run(&b.root.join(cwd), args), &base(&b));
    (a, want, b, got)
}

/// f の写し 2 つに Go と core を当て、出力と結果のツリーを比べる。
fn same_write(f: &Fixture, cwd: &str, args: &[&str]) -> Out {
    let what = format!("desk {}", args.join(" "));
    let (a, want, b, got) = run_both(f, cwd, args);
    assert_eq!(got, want, "{what} が Go と違う");
    same_trees(&a.root, &b.root, &what);
    got
}

fn same_writes(f: &Fixture, cases: &[&[&str]]) {
    for args in cases {
        same_write(f, "", args);
    }
}

// init

/// 空の一時ディレクトリ 2 つで setup してから init を流し、比べる。
fn same_init(setup: impl Fn(&Path), cwd: &str, args: &[&str]) -> Out {
    let (da, db) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (a, b) = (
        da.path().canonicalize().unwrap(),
        db.path().canonicalize().unwrap(),
    );
    setup(&a);
    setup(&b);
    fs::create_dir_all(a.join(cwd)).unwrap();
    fs::create_dir_all(b.join(cwd)).unwrap();
    let want = norm(go_run(&a.join(cwd), args), &a);
    let got = norm(cli::run(&b.join(cwd), args), &b);
    let what = format!("desk {}", args.join(" "));
    assert_eq!(got, want, "{what} が Go と違う");
    same_trees(&a.join("desk"), &b.join("desk"), &what);
    got
}

fn git_in(dir: &Path, args: &[&str]) {
    let st = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=desk", "-c", "user.email=desk@example.com"])
        .args(args)
        .output()
        .unwrap();
    assert!(st.status.success(), "git {args:?}");
}

#[test]
fn init_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let out = same_init(|_| {}, "", &["init", "desk"]);
    assert!(out.stdout.contains("最初のコミットを作った"), "{out:?}");
    same_init(|_| {}, "desk", &["init"]);
    // もうルート
    same_init(
        |d| {
            fs::create_dir_all(d.join("desk")).unwrap();
            fs::write(d.join("desk/.desk"), "").unwrap();
        },
        "",
        &["init", "desk"],
    );
    // git のあるディレクトリ（あるファイルは書き換えず、コミットしない）
    let out = same_init(
        |d| {
            let r = d.join("desk");
            fs::create_dir_all(&r).unwrap();
            fs::write(r.join("README.md"), "# 前からある\n").unwrap();
            git_in(&r, &["init", "-q", "-b", "main"]);
            git_in(&r, &["add", "README.md"]);
            git_in(&r, &["commit", "-q", "-m", "first"]);
        },
        "",
        &["init", "desk"],
    );
    assert!(out.stdout.contains("コミットはしていない"), "{out:?}");
    // ツリーの中・ファイル・引数が多い
    for args in [
        &["init", "desk/projects/misc/nested"][..],
        &["init", "desk/README.md"],
        &["init", "a", "b"],
    ] {
        let out = same_init(
            |d| {
                let r = d.join("desk");
                fs::create_dir_all(r.join("projects/misc")).unwrap();
                fs::write(r.join(".desk"), "").unwrap();
                fs::write(r.join("README.md"), "# x\n").unwrap();
            },
            "",
            args,
        );
        assert_ne!(out.code, 0, "{args:?}");
    }
}

// new

#[test]
fn new_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = recipe_tree();
    f.write(
        "projects/recipe-app/archive/tasks/2026-09-07-favorites.md",
        &task_doc("done", "", "お気に入り"),
    );
    f.write(
        "projects/misc/artifacts/print-layout.md",
        "---\ntasks: []\n---\n# 別の\n",
    );
    f.write(
        "archive/projects/old/tasks/2026-01-01-old.md",
        &task_doc("done", "", "古い"),
    );
    f.write("archive/projects/gone/README.md", "# gone\n");
    let ok: &[&[&str]] = &[
        &["new", "project", "recipe-web"],
        &[
            "new",
            "task",
            "2026-09-08-meal-plan",
            "--project",
            "recipe-app",
            "--title",
            "献立を組む",
        ],
        &["new", "task", "shopping-list", "--project", "recipe-app"],
        &[
            "new",
            "task",
            "2026-11-10-decide-paper-size",
            "--parent",
            "2026-09-07-meal-planner",
            "--needs-human",
        ],
        &[
            "new",
            "task",
            "x",
            "--parent",
            "2026-09-07-meal-planner",
            "--project",
            "misc",
        ],
        &[
            "new",
            "artifact",
            "print-columns",
            "--parent",
            "2026-11-09-printable-menu",
        ],
        &["new", "artifact", "notes", "--project", "misc"],
        &["new", "knowledge", "reproduce-before-fixing"],
        &["new", "knowledge", "api-retry", "--project", "recipe-app"],
        &["new", "principle", "keep-it-small"],
        &[
            "new",
            "wiki",
            "meal-planner/print",
            "--project",
            "recipe-app",
        ],
        &[
            "new",
            "wiki",
            "meal-planner/README",
            "--project",
            "recipe-app",
        ],
        &["new", "inbox", "slow-list", "--project", "misc"],
        &[
            "new",
            "library",
            "2026-10-01-scroll",
            "--project",
            "misc",
            "--title",
            "無限スクロール",
        ],
        &[
            "new",
            "material",
            "patched-each-screen",
            "--project",
            "recipe-app",
        ],
    ];
    for args in ok {
        let out = same_write(&f, "", args);
        assert_eq!(out.code, 0, "{args:?}: {out:?}");
    }
    let bad: &[&[&str]] = &[
        &["new", "project", "recipe-app"],
        &["new", "project", "gone"],
        &["new", "project", "Bad_Name"],
        &["new", "foo", "x"],
        &["new", "readme", "x"],
        &["new", "artifact", "x", "--needs-human", "--project", "misc"],
        &["new", "wiki", "x", "--parent", "2026-09-07-meal-planner"],
        &["new", "task", "x"],
        &["new", "task", "x", "--project", "nothing"],
        &["new", "task", "x", "--project", "a/b"],
        &["new", "task", "Bad_Slug", "--project", "recipe-app"],
        &["new", "wiki", "a/Bad", "--project", "recipe-app"],
        &[
            "new",
            "task",
            "2026-09-07-favorites",
            "--project",
            "recipe-app",
        ],
        &[
            "new",
            "task",
            "2026-09-07-meal-planner",
            "--project",
            "recipe-app",
        ],
        &["new", "task", "x", "--parent", "print-layout"],
        &[
            "new",
            "task",
            "x",
            "--parent",
            "projects/recipe-app/artifacts/print-layout.md",
        ],
        &["new", "task", "x", "--parent", "nothing"],
        &["new", "task", "x", "--parent", "2026-01-01-old"],
    ];
    for args in bad {
        let out = same_write(&f, "", args);
        assert_ne!(out.code, 0, "{args:?}");
    }
    // 作業ディレクトリのプロジェクトで名前を絞る
    same_write(
        &f,
        "projects/misc",
        &[
            "new",
            "artifact",
            "y",
            "--parent",
            "../recipe-app/tasks/2026-09-07-meal-planner.md",
        ],
    );
}

// archive

/// archive と mv のツリー（git にコミットしたもの。コミットしていないファイルも 1 つ置く）。
fn move_tree() -> Fixture {
    let f = recipe_tree();
    f.write(
        "projects/misc/artifacts/notes.md",
        "---\ntasks: []\n---\n\n[概要](../../recipe-app/wiki/meal-planner/overview.md)\n",
    );
    f.write(
        "projects/recipe-app/tasks/2026-11-11-no-status.md",
        "---\nneeds_human: false\nparent: 2026-11-09-printable-menu.md\n---\n\n# status が無い\n",
    );
    f.write(
        "projects/recipe-app/tasks/2026-11-12-empty-status.md",
        "---\nstatus:\nneeds_human: false\n---\n\n# status が空\n\n[印刷](2026-11-09-printable-menu.md#目的)\n",
    );
    f.write(
        "projects/recipe-app/tasks/2026-11-13-quoted.md",
        "---\nstatus: \"todo\"\nneeds_human: false\ndepends_on: [2026-11-12-empty-status.md, 2026-11-11-no-status.md]\n---\n\n# 引用した status\n",
    );
    f.write(
        "projects/recipe-app/tasks/2026-11-14-no-fm.md",
        "# frontmatter が無い\n\n[献立](2026-09-07-meal-planner.md) と [印刷](./2026-11-09-printable-menu.md)\n",
    );
    // status が null（Go は値の字だけを置き換える）
    for (name, status) in [
        ("2026-11-15-tilde", "status: ~ # まだ"),
        ("2026-11-16-null", "status: null"),
        ("2026-11-17-next-line", "status:\n  ~"),
        ("2026-11-18-list", "status: [todo]"),
    ] {
        f.write(
            &format!("projects/recipe-app/tasks/{name}.md"),
            &format!("---\n{status}\nneeds_human: false\n---\n\n# {name}\n"),
        );
    }
    f.write(
        "projects/recipe-app/archive/tasks/2026-08-01-old.md",
        &task_doc("done", "", "前のもの"),
    );
    f.commit_all("2026-11-01");
    f.write(
        "projects/recipe-app/artifacts/untracked.md",
        "---\ntasks: [../tasks/2026-11-09-printable-menu.md]\n---\n\n# コミットしていない\n",
    );
    f
}

#[test]
fn archive_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = move_tree();
    let task = "projects/recipe-app/tasks/2026-11-09-printable-menu.md";
    let docs = [
        "projects/recipe-app/artifacts/print-layout.md",
        "projects/recipe-app/artifacts/assets/print-sample.png",
        "projects/recipe-app/artifacts/untracked.md",
    ];
    let mut all = vec!["archive", task];
    all.extend(docs);
    let mut dry = all.clone();
    dry.push("--dry-run");
    let out = same_write(&f, "", &dry);
    assert!(out.stdout.contains("書き換える:"), "{out:?}");
    same_write(&f, "", &all);
    let mut dropped = all.clone();
    dropped.extend(["--status", "dropped"]);
    same_write(&f, "", &dropped);
    same_writes(
        &f,
        &[
            &["archive", "2026-11-11-no-status", "--dry-run"],
            &["archive", "2026-11-11-no-status"],
            &["archive", "2026-11-12-empty-status", "--dry-run"],
            &["archive", "2026-11-12-empty-status", "2026-11-13-quoted"],
            &[
                "archive",
                "2026-11-13-quoted",
                "--status",
                "dropped",
                "--dry-run",
            ],
            &["archive", "2026-11-14-no-fm", "--dry-run"],
            &["archive", "2026-11-15-tilde", "--dry-run"],
            &["archive", "2026-11-15-tilde"],
            &["archive", "2026-11-16-null", "--status", "dropped"],
            &["archive", "2026-11-17-next-line", "--dry-run"],
            &["archive", "2026-11-17-next-line"],
            &["archive", "2026-11-18-list", "--dry-run"],
            &["archive", "2026-11-18-list"],
            &["archive", "2026-11-14-no-fm"],
            &["archive", "2026-08-01-old"],
            &[
                "archive",
                "2026-08-01-old",
                "--status",
                "dropped",
                "--dry-run",
            ],
            &["archive", "2026-08-01-old", "--status", "dropped"],
            &["archive", task, task],
            &["archive", "2026-08-01-old", task],
            &["archive", "--project", "recipe-app", "--dry-run"],
            &["archive", "--project", "recipe-app"],
            &["archive", "--project", "misc"],
        ],
    );
    let bad: &[&[&str]] = &[
        &[
            "archive",
            "projects/recipe-app/wiki/meal-planner/overview.md",
        ],
        &["archive", "2026-08-01-old", "nothing"],
        &["archive", "projects/recipe-app/tasks"],
        &["archive", task, "--status", "closed"],
        &["archive", "--project", "nothing"],
        &["archive", "--project", "recipe-app", "--status", "done"],
        &["archive", "--project", "recipe-app", task],
    ];
    for args in bad {
        let out = same_write(&f, "", args);
        assert_ne!(out.code, 0, "{args:?}");
    }
    // 移す先がもうある
    let g = move_tree();
    g.write("archive/projects/misc/README.md", "# misc\n");
    g.write(
        "projects/recipe-app/archive/tasks/2026-11-09-printable-menu.md",
        &task_doc("done", "", "先にある"),
    );
    for args in [&["archive", "--project", "misc"][..], &["archive", task]] {
        let out = same_write(&g, "", args);
        assert_ne!(out.code, 0, "{args:?}");
    }
}

// mv

#[test]
fn mv_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = move_tree();
    let layout = "projects/recipe-app/artifacts/print-layout.md";
    let ok: &[&[&str]] = &[
        &[
            "mv",
            layout,
            "projects/recipe-app/artifacts/print-columns.md",
            "--dry-run",
        ],
        &[
            "mv",
            layout,
            "projects/recipe-app/artifacts/print-columns.md",
        ],
        &[
            "mv",
            "projects/recipe-app/knowledge/api-error-format.md",
            "knowledge/",
        ],
        &[
            "mv",
            "projects/recipe-app/knowledge/api-error-format.md",
            "knowledge",
        ],
        &[
            "mv",
            "projects/recipe-app/wiki/meal-planner",
            "projects/recipe-app/wiki/planner",
        ],
        &[
            "mv",
            "projects/recipe-app/wiki/meal-planner",
            "projects/misc/wiki/",
        ],
        &[
            "mv",
            layout,
            "projects/recipe-app/artifacts/untracked.md",
            "projects/misc/artifacts/",
        ],
        &["mv", "2026-11-14-no-fm", "projects/misc/tasks/"],
        &[
            "mv",
            "projects/recipe-app/archive/tasks/2026-08-01-old.md",
            "projects/recipe-app/tasks/",
        ],
        &["mv", "projects/recipe-app/tasks", "projects/misc/"],
    ];
    for args in ok {
        let out = same_write(&f, "", args);
        assert_eq!(out.code, 0, "{args:?}: {out:?}");
    }
    let bad: &[&[&str]] = &[
        &["mv", layout, "projects/recipe-app/drafts/"],
        &["mv", layout, "projects/recipe-app/artifacts/"],
        &["mv", layout, "/tmp/x.md"],
        &["mv", layout, "../outside/"],
        &["mv", layout, "projects/recipe-app/artifacts/untracked.md"],
        &[
            "mv",
            layout,
            "projects/recipe-app/tasks/2026-09-07-meal-planner.md",
            "x.md",
        ],
        &["mv", "projects/recipe-app/inbox", "projects/misc/"],
        &["mv", "nothing", "projects/misc/"],
        &["mv", "print-layout", "x.md"],
    ];
    for args in bad {
        let out = same_write(&f, "", args);
        assert_ne!(out.code, 0, "{args:?}");
    }
    // 作業ディレクトリからの先
    same_write(
        &f,
        "projects/recipe-app/artifacts",
        &["mv", "print-layout.md", "print-grid.md"],
    );
    same_write(
        &f,
        "projects/recipe-app",
        &["mv", "artifacts/print-layout.md", "../misc/artifacts/"],
    );
}

// check --fix

/// check --fix を比べる。Go と変えたところ（frontmatter の値の行）は Go の側を直してから比べ、直した数を返す。
fn same_fix(f: &Fixture, args: &[&str]) -> (Out, usize) {
    let what = format!("desk {}", args.join(" "));
    let (a, mut want, b, got) = run_both(f, "", args);
    let n = if args.contains(&"--json") {
        fix_go_fm_lines(&a.root, &mut want)
    } else {
        0
    };
    assert_eq!(got, want, "{what} が Go と違う");
    same_trees(&a.root, &b.root, &what);
    (got, n)
}

/// Go の TestCheckFix のツリー。
fn fix_tree() -> Fixture {
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
    f
}

#[test]
fn check_fix_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = fix_tree();
    let (out, lines) = same_fix(&f, &["check", "--fix", "--json"]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(lines > 0, "frontmatter の値の行の例外を確かめていない");
    same_fix(
        &f,
        &["check", "--fix", "--json", "--name", "2026-11-04-closed"],
    );
    same_fix(&f, &["check", "--fix", "--json", "--project", "misc"]);

    // 全部直る（表が要らない）
    let g = recipe_tree();
    g.write(
        "projects/recipe-app/tasks/2026-11-04-closed.md",
        &(task_doc("done", "", "閉じた") + "\n[設計](../artifacts/print-layout.md)\n"),
    );
    g.write(
        "projects/recipe-app/artifacts/moved-by-hand.md",
        "---\ntasks: []\n---\n\n[概要](overview.md)\n",
    );
    g.write(
        "projects/recipe-app/artifacts/no-tasks.md",
        "# tasks が無い\n",
    );
    let (out, _) = same_fix(&g, &["check", "--fix"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(out.stdout.contains("直した:"), "{out:?}");
    // 直すものが無い
    same_fix(&recipe_tree(), &["check", "--fix"]);

    // 手で直すものが残る（Go の TestCheckFixLeavesManualFindings）
    let h = recipe_tree();
    h.write("projects/recipe-app/knowledge/empty.md", "# 中身だけ\n");
    same_fix(&h, &["check", "--fix", "--json"]);
}

#[test]
fn check_fix_on_check_tree_matches_go() {
    if go_desk().is_none() {
        return;
    }
    let f = check_tree();
    f.commit_all("2026-11-01");
    let (out, _) = same_fix(&f, &["check", "--fix", "--json"]);
    assert_eq!(out.code, 1);
}
