//! CLI の入口: 使い方（`-h`・`--help`・`help`）、引数の誤り、書き出しの失敗。読み方と文言は clap に任せるので、
//! 文言そのものは見ず、出る場所と終了コード（0 = 使い方を出した、2 = 引数の誤り、1 = 失敗）と、ツリーを変えない
//! ことを見る。

mod common;

use std::process::{Command, Stdio};

use common::run::{desk, desk_with_stdin};
use common::{edge_tree, git, recipe_tree, with_git};

const COMMANDS: &[&str] = &[
    "init", "root", "new", "ls", "graph", "read", "check", "rules", "archive", "mv",
];

#[test]
fn help_goes_to_stdout() {
    let f = recipe_tree();
    for args in [&["-h"][..], &["--help"], &["help"]] {
        let o = desk(&f.root, args);
        assert!(
            o.code == 0 && o.stdout.contains("archive"),
            "{args:?}: {o:?}"
        );
    }
    for c in COMMANDS {
        for h in ["-h", "--help"] {
            let o = desk(&f.root, &[c, h]);
            assert!(
                o.code == 0 && o.stdout.contains(&format!("desk {c}")),
                "desk {c} {h}: {o:?}"
            );
        }
        let o = desk(&f.root, &["help", c]);
        assert!(o.code == 0 && !o.stdout.is_empty(), "desk help {c}: {o:?}");
    }
}

#[test]
fn no_command_shows_help_as_error() {
    let f = recipe_tree();
    let o = desk(&f.root, &[]);
    assert!(o.code == 2 && o.stderr.contains("archive"), "{o:?}");
}

#[test]
fn bad_arguments_exit_2_without_touching_tree() {
    let f = with_git(recipe_tree());
    let before = git(&f.root, &["status", "--porcelain", "--untracked-files=all"]);
    let mut cases: Vec<Vec<&str>> = vec![vec!["nothing"], vec!["--nothing"]];
    for c in COMMANDS {
        cases.push(vec![c, "--nothing"]);
        cases.push(vec![c, "-x"]);
    }
    cases.extend([
        // 値が要るのに無い・読めない
        vec!["ls", "--where"],
        vec!["ls", "--where", "=x"],
        vec!["ls", "--where", "a?b"],
        vec!["ls", "--modified-before", "abc"],
        vec!["ls", "--modified-after", "-1"],
        vec!["ls", "--modified-after", "2026-13-01"],
        vec!["ls", "--level", "3"],
        vec!["read", "x", "--level", "3"],
        vec!["graph", "x", "--level", "-1"],
        vec!["graph", "x", "--depth", "0"],
        vec!["graph", "x", "--depth", "x"],
        vec!["archive", "x", "--status", "todo"],
        vec!["new", "task", "x", "--project"],
        // 引数の数と組み合わせ
        vec!["root", "x"],
        vec!["init", "a", "b"],
        vec!["new"],
        vec!["new", "task"],
        vec!["new", "a", "b", "c"],
        vec!["ls", "task", "nothing"],
        vec!["ls", "--json", "--paths"],
        vec!["ls", "--paths", "--level", "0"],
        vec!["ls", "--json", "--level", "1"],
        vec!["ls", "--fields", "path", "--paths"],
        vec!["graph"],
        vec!["graph", "a", "b"],
        vec!["graph", "x", "--in", "--out"],
        vec!["graph", "--all", "x"],
        vec!["graph", "--all", "--in"],
        vec!["graph", "--all", "--depth", "2"],
        vec!["graph", "--all", "--level", "0"],
        vec!["graph", "README.md", "--format", "nothing"],
        vec!["read"],
        vec!["read", "x", "--outline", "--section", "a"],
        vec!["check", "x"],
        vec!["check", "--rules"],
        vec!["rules", "x"],
        vec!["archive"],
        vec!["archive", "x", "--project", "misc"],
        vec!["archive", "--project", "misc", "--status", "done"],
        vec!["mv"],
        vec!["mv", "a"],
        // clap が読んだあとに desk が見る誤り
        vec!["new", "nothing", "x", "--project", "misc"],
        vec!["graph", "--all", "--format", "tree"],
        vec!["graph", "--all", "--format", "paths"],
        vec!["graph", "README.md", "--format", "edges"],
    ]);
    for args in &cases {
        let o = desk(&f.root, args);
        assert!(
            o.code == 2 && o.stdout.is_empty() && !o.stderr.is_empty(),
            "desk {}: {o:?}",
            args.join(" ")
        );
    }
    let o = desk_with_stdin(&f.root, &["read", "-"], "\n \n");
    assert!(o.code == 2 && !o.stderr.is_empty(), "空の標準入力: {o:?}");
    // 失敗（終了コード 1）
    for args in [&["read", "nothing"][..], &["graph", "nothing"]] {
        let o = desk(&f.root, args);
        assert!(
            o.code == 1 && o.stderr.starts_with("desk: "),
            "{args:?}: {o:?}"
        );
    }
    assert_eq!(
        git(&f.root, &["status", "--porcelain", "--untracked-files=all"]),
        before,
        "止まるはずのコマンドがツリーを変えた"
    );
}

/// 読む側が先に閉じたら（`| head` など）、黙って 0 で終える。
#[test]
fn closed_stdout_is_not_an_error() {
    let f = recipe_tree();
    for args in [&["ls", "--level", "1"][..], &["graph", "--all"], &["rules"]] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_desk"))
            .args(args)
            .current_dir(&f.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        drop(child.stdout.take());
        let o = child.wait_with_output().unwrap();
        assert!(
            o.status.code() == Some(0) && o.stderr.is_empty(),
            "desk {}: {:?} {}",
            args.join(" "),
            o.status,
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

/// 読み方がずれやすいツリーでも、止まらずに読む（パニックしない）。
#[test]
fn odd_tree_reads_without_panicking() {
    let f = edge_tree();
    for args in [
        &["ls", "--json", "--archived"][..],
        &["ls", "--archived"],
        &["graph", "--all"],
        &["graph", "--all", "--format", "dot"],
        &["check", "--json"],
    ] {
        let o = desk(&f.root, args);
        assert!(
            o.code <= 1 && !o.stdout.is_empty() && !o.stderr.contains("panicked"),
            "desk {}: {o:?}",
            args.join(" ")
        );
    }
}
