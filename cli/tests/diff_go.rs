//! 手で動かす比べ: 手元の実際のツリーの複製を Go の CLI と Rust の CLI で読み、意味が同じかを見る。
//! Go の CLI を消す切り替え（R5）で、Go と一緒に消す。
//!
//! バイトでは比べない（文言や並び、行の数え方は変えてよい）。比べるのは:
//!
//! - `ls --json --archived`: パスごとの 1 件（キーの順は見ない）
//! - `graph --all`: 辺の集まり
//! - `check --json`: 見つけたもの（パス・何を・どう直すか・直せるか）の集まり。行と「何が」の文言は見ない
//! - `check --fix`: 直したあとのツリーの中身
//!
//! DESK_DIFF_TREE にツリーのルートを渡したときだけ動く（Go が要る）。結果はリポジトリに入れない。
//!
//! ```sh
//! DESK_DIFF_TREE=~/desk cargo test -p desk-cli --test diff_go -- --nocapture
//! ```

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::Fixture;
use common::run::{Out, desk, run_bin};
use serde_json::Value;

/// Go の CLI をビルドする。
fn go_desk() -> PathBuf {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("desk-go");
    let status = Command::new("go")
        .args(["build", "-o"])
        .arg(&out)
        .arg("./cli/cmd/desk")
        .current_dir(repo)
        .status()
        .expect("go を動かせない（Go の CLI との比べには Go が要る）");
    assert!(status.success(), "Go の CLI をビルドできない");
    out
}

#[test]
fn real_tree_means_the_same_as_go() {
    let Some(src) = std::env::var_os("DESK_DIFF_TREE") else {
        return;
    };
    let bin = go_desk();
    let f = Fixture::copy_of(Path::new(&src));
    let go = |args: &[&str]| run_bin(&bin, &f.root, args, "");
    let rust = |args: &[&str]| desk(&f.root, args);
    let mut diffs = Vec::new();

    let args = ["ls", "--json", "--archived"];
    diffs.extend(differ("ls", entries(&go(&args)), entries(&rust(&args))));

    let args = ["graph", "--all"];
    diffs.extend(differ("graph", edges(&go(&args)), edges(&rust(&args))));

    let args = ["check", "--json"];
    diffs.extend(differ(
        "check",
        findings(&go(&args)),
        findings(&rust(&args)),
    ));

    let (gf, rf) = (f.duplicate(), f.duplicate());
    run_bin(&bin, &gf.root, &["check", "--fix"], "");
    desk(&rf.root, &["check", "--fix"]);
    diffs.extend(differ("check --fix", files(&gf.root), files(&rf.root)));

    assert!(diffs.is_empty(), "Go と意味が違う:\n{}", diffs.join("\n"));
}

/// 片方にだけあるものを並べる。
fn differ<K: Ord + std::fmt::Debug, V: PartialEq + std::fmt::Debug>(
    what: &str,
    go: BTreeMap<K, V>,
    rust: BTreeMap<K, V>,
) -> Vec<String> {
    let keys: BTreeSet<&K> = go.keys().chain(rust.keys()).collect();
    keys.into_iter()
        .filter(|k| go.get(k) != rust.get(k))
        .map(|k| {
            format!(
                "{what} {k:?}\n  Go:   {:?}\n  Rust: {:?}",
                go.get(k),
                rust.get(k)
            )
        })
        .collect()
}

/// 終了コードが 0 か 1（check の見つけたもの）で、標準出力があるものとして読む。
fn stdout(o: &Out) -> &str {
    assert!(o.code <= 1, "止まった: {o:?}");
    &o.stdout
}

/// `ls --json` をパスごとに。
fn entries(o: &Out) -> BTreeMap<String, Value> {
    let v: Vec<Value> = serde_json::from_str(stdout(o)).unwrap();
    v.into_iter()
        .map(|e| (e["path"].as_str().unwrap().to_string(), e))
        .collect()
}

/// `graph --all` の表を辺の集まり（数）に。
fn edges(o: &Out) -> BTreeMap<Vec<String>, usize> {
    let mut m = BTreeMap::new();
    for l in stdout(o).lines().skip(1) {
        *m.entry(l.split_whitespace().map(String::from).collect())
            .or_default() += 1;
    }
    m
}

/// `check --json` を、行と文言を除いた見つけたものの集まり（数）に。
fn findings(o: &Out) -> BTreeMap<(String, String, String, bool), usize> {
    let v: Vec<Value> = serde_json::from_str(stdout(o)).unwrap();
    let mut m = BTreeMap::new();
    for x in v {
        let s = |k: &str| x[k].as_str().unwrap().to_string();
        *m.entry((s("path"), s("category"), s("fix"), x["fixable"] == true))
            .or_default() += 1;
    }
    m
}

/// ツリーの中のファイル（.git の外）を、ルートからのパスと中身に。
fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut m = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                m.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    m
}
