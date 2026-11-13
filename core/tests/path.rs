//! パスから分かること（種類・プロジェクト・archive の中か）、archive との間のパス、名前の決まり。

use desk_core::path::{archive_path, name_problem, unarchive_path};
use desk_core::{Kind, classify};

#[test]
fn classify_paths() {
    struct Case {
        path: &'static str,
        kind: Option<Kind>,
        project: &'static str,
        archived: bool,
        project_archived: bool,
        problem: bool,
    }
    const fn ok(path: &'static str, kind: Kind, project: &'static str) -> Case {
        Case {
            path,
            kind: Some(kind),
            project,
            archived: false,
            project_archived: false,
            problem: false,
        }
    }
    const fn bad(path: &'static str, kind: Option<Kind>, project: &'static str) -> Case {
        Case {
            path,
            kind,
            project,
            archived: false,
            project_archived: false,
            problem: true,
        }
    }
    let tests = [
        ok("README.md", Kind::Readme, ""),
        ok("principles/x.md", Kind::Principle, ""),
        ok("knowledge/x.md", Kind::Knowledge, ""),
        ok("assets/a.png", Kind::Asset, ""),
        ok("projects/app/README.md", Kind::Readme, "app"),
        ok("projects/app/tasks/2026-01-02-x.md", Kind::Task, "app"),
        ok("projects/app/artifacts/x.md", Kind::Artifact, "app"),
        ok("projects/app/artifacts/assets/a.png", Kind::Asset, "app"),
        ok("projects/app/wiki/a/b/c.md", Kind::Wiki, "app"),
        Case {
            archived: true,
            ..ok(
                "projects/app/archive/tasks/2026-01-02-x.md",
                Kind::Task,
                "app",
            )
        },
        Case {
            archived: true,
            ..ok(
                "projects/app/archive/artifacts/assets/a.png",
                Kind::Asset,
                "app",
            )
        },
        ok(
            "projects/misc/inbox/2026-01-02-memo.md",
            Kind::Inbox,
            "misc",
        ),
        Case {
            archived: true,
            project_archived: true,
            ..ok(
                "archive/projects/app/tasks/2026-01-02-x.md",
                Kind::Task,
                "app",
            )
        },
        Case {
            archived: true,
            project_archived: true,
            ..ok(
                "archive/projects/app/archive/tasks/2026-01-02-x.md",
                Kind::Task,
                "app",
            )
        },
        bad("notes.md", None, ""),
        bad("app/tasks/2026-01-02-x.md", None, ""),
        bad("projects/x.md", None, ""),
        bad("projects/app/notes.md", None, "app"),
        bad("projects/app/drafts/x.md", None, "app"),
        bad("projects/app/tasks/sub/x.md", Some(Kind::Task), "app"),
        bad("projects/app/tasks/a.png", Some(Kind::Task), "app"),
        Case {
            archived: true,
            ..bad("projects/app/archive/wiki/x.md", None, "app")
        },
        bad("archive/tasks/x.md", None, ""),
    ];
    for tt in tests {
        let got = classify(tt.path);
        assert!(
            got.kind == tt.kind
                && got.project == tt.project
                && got.archived == tt.archived
                && got.project_archived == tt.project_archived
                && got.problem.is_some() == tt.problem,
            "classify({:?}) = {got:?}",
            tt.path
        );
    }
}

#[test]
fn archive_paths() {
    let tests = [
        ("projects/app/tasks/x.md", "projects/app/archive/tasks/x.md"),
        (
            "projects/app/artifacts/assets/a.png",
            "projects/app/archive/artifacts/assets/a.png",
        ),
        (
            "archive/projects/app/tasks/x.md",
            "archive/projects/app/tasks/x.md",
        ),
        ("principles/x.md", "principles/x.md"),
    ];
    for (from, to) in tests {
        assert_eq!(archive_path(&classify(from)), to, "archive_path({from:?})");
        if from != to {
            assert_eq!(
                unarchive_path(&classify(to)),
                from,
                "unarchive_path({to:?})"
            );
        }
    }
}

#[test]
fn name_problems() {
    let tests = [
        ("projects/app/tasks/2026-11-09-x.md", false),
        ("projects/app/tasks/x.md", true),
        ("projects/app/tasks/2026-13-40-x.md", true),
        ("projects/app/artifacts/x.md", false),
        ("projects/app/materials/x.md", true),
    ];
    for (path, problem) in tests {
        let got = name_problem(&classify(path));
        assert_eq!(got.is_some(), problem, "name_problem({path:?}) = {got:?}");
    }
}
