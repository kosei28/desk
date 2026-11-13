//! CLI のテストの道具。架空のツリーは core のテストと同じもの（core/tests/common/mod.rs が正本）を使う。

// テストのファイルごとに使うものが違う
#![allow(dead_code)]

#[path = "../../../core/tests/common/mod.rs"]
mod fixtures;
pub mod run;

pub use fixtures::*;

use std::fs;
use std::path::Path;
use std::process::Command;

/// git を cwd で動かす（ユーザーの設定は読まない）。失敗したら止める。
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = run::isolate_git(Command::new("git").arg("-C").arg(dir).args(args))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// desk init の後と同じ形にする: git のリポジトリにし、init が置くファイルだけを最初のコミットにする
/// （そのあとに書いたものはコミットしない）。
pub fn with_git(f: Fixture) -> Fixture {
    git(&f.root, &["init", "-q", "-b", "main"]);
    git(
        &f.root,
        &[
            "add",
            ".desk",
            ".gitignore",
            "README.md",
            "projects/misc/README.md",
        ],
    );
    git(&f.root, &["commit", "-q", "-m", "docs: desk を作る"]);
    f
}

/// 全部をコミットする。
pub fn git_add_all(root: &Path) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
}

/// ルートからのパスのファイルを読む。
pub fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

pub fn exists(root: &Path, rel: &str) -> bool {
    root.join(rel).exists()
}

/// ツリーの外で動かしたコマンドが、何も書かずに「ツリーの中ではない」で止まるか。
pub fn outside_tree_fails(cmds: &[&[&str]]) {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().canonicalize().unwrap();
    for args in cmds {
        let o = run::desk(&outside, args);
        assert!(
            o.code != 0 && o.stderr.contains("ツリーの中ではない"),
            "desk {args:?} がツリーの外で止まらない: {o:?}"
        );
    }
    let left: Vec<_> = fs::read_dir(&outside).unwrap().collect();
    assert!(left.is_empty(), "ツリーの外に書いている: {left:?}");
}
