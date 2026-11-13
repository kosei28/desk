//! 差分テスト: 同じ架空のツリーを Go の CLI と desk-core で読み、出るものを比べる。
//! Go の CLI を消す切り替え（R5）で、Go と一緒に消す。
//!
//! CLI の段（R4）の前なので、Go は CLI を流し、Rust は core を呼んで、CLI が出すのと同じ形にして比べる。
//! - `ls --json`（と `--archived`）: core の `Tree::entries` を serialize したものとバイトで比べる
//! - `graph --all`: 辺の並びを比べる（Go は `--format dot` で出して読む。表の整形は CLI の段で比べる）
//!
//! Go が要る。`go` が無ければ失敗する（Go を入れずに飛ばすなら DESK_SKIP_GO_DIFF=1）。

mod common;

use std::path::Path;

use common::go::{go, go_desk, go_unquote, parse_dot};
use common::{Fixture, edge_tree, exception_tree, recipe_tree};
use desk_core::Tree;
use serde_json::Value;

/// `ls --json` と同じ形（2 文字の字下げ、末尾に改行）。
fn rust_ls_json(tree: &Tree, archived: bool) -> String {
    serde_json::to_string_pretty(&tree.entries(&[], None, archived)).unwrap() + "\n"
}

/// ツリーを両方で読み、ls --json（archive を入れる・入れない）と graph --all を比べる。
fn compare(f: &Fixture) {
    if go_desk().is_none() {
        return;
    }
    let tree = Tree::load(&f.root).unwrap();
    for archived in [false, true] {
        let args: &[&str] = if archived {
            &["ls", "--json", "--archived"]
        } else {
            &["ls", "--json"]
        };
        let want = go(&f.root, args);
        let got = rust_ls_json(&tree, archived);
        assert_eq!(got, want, "desk {} が Go と違う", args.join(" "));
    }
    let want = parse_dot(&go(&f.root, &["graph", "--all", "--format", "dot"]));
    assert_eq!(tree.edges(), want, "desk graph --all が Go と違う");
}

#[test]
fn recipe_tree_matches_go() {
    compare(&recipe_tree());
}

#[test]
fn edge_tree_matches_go() {
    compare(&edge_tree());
}

#[test]
fn empty_tree_matches_go() {
    compare(&Fixture::new());
}

/// 手元の実際のツリーの複製でも比べる（DESK_DIFF_TREE にツリーのルートを渡したときだけ）。
/// 結果はリポジトリに入れない。
#[test]
fn real_tree_matches_go() {
    let Some(src) = std::env::var_os("DESK_DIFF_TREE") else {
        return;
    };
    compare(&Fixture::copy_of(Path::new(&src)));
}

/// Go と読み方を変えたところ。1 件ずつ、Go の値と Rust の値と理由を残す（理由はタスクのログにもある）。
struct Exception {
    path: &'static str,
    key: &'static str,
    go: &'static str,
    rust: &'static str,
    why: &'static str,
}

const EXCEPTIONS: &[Exception] = &[
    Exception {
        path: "projects/alpha/library/2026-11-01-typed.md",
        key: "tags",
        go: r#"[2026, true, null, 1.5, "rust"]"#,
        rust: r#"["2026", "true", "1.5", "rust"]"#,
        why: "配列の値も書いてあるとおりの文字列にする。Go はいちばん上の値は文字列、配列の中だけ YAML の型で出していた。null の要素は Go の List と同じく入れない",
    },
    Exception {
        path: "projects/alpha/library/2026-11-01-typed.md",
        key: "meta",
        go: r#"{"a": "b"}"#,
        rust: "null",
        why: "frontmatter の値は文字列か文字列の配列で、入れ子の対応は持たない（Go の Str・List と同じく空として扱う）",
    },
    Exception {
        path: "projects/alpha/library/2026-11-01-typed.md",
        key: "nested",
        go: r#"[["a"], "b"]"#,
        rust: r#"["b"]"#,
        why: "配列の要素のうち文字列でないもの（入れ子）は入れない（Go の List と同じ）",
    },
];

#[test]
fn exception_tree_differs_only_as_listed() {
    if go_desk().is_none() {
        return;
    }
    let f = exception_tree();
    let tree = Tree::load(&f.root).unwrap();
    let mut want: Value = serde_json::from_str(&go(&f.root, &["ls", "--json"])).unwrap();
    for e in EXCEPTIONS {
        let item = want
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["path"] == e.path)
            .unwrap_or_else(|| panic!("{} が Go の一覧に無い", e.path));
        let v = &mut item["frontmatter"][e.key];
        let go: Value = serde_json::from_str(e.go).unwrap();
        assert_eq!(
            *v, go,
            "{} の {}: Go の値が控えと違う（{}）",
            e.path, e.key, e.why
        );
        *v = serde_json::from_str(e.rust).unwrap();
    }
    let got: Value = serde_json::to_value(tree.entries(&[], None, false)).unwrap();
    assert_eq!(got, want, "例外として控えたところのほかにも Go と違う");
    let want = parse_dot(&go(&f.root, &["graph", "--all", "--format", "dot"]));
    assert_eq!(tree.edges(), want, "desk graph --all が Go と違う");
}

#[test]
fn go_unquote_reads_go_quoting() {
    assert_eq!(
        go_unquote(r#"a\"b\\c\té\x01"] rest"#),
        ("a\"b\\c\té\x01".to_string(), "] rest")
    );
}
