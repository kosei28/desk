//! 差分テスト: 同じ架空のツリーを Go の CLI と desk-core で読み、出るものを比べる。
//! Go の CLI を消す切り替え（R5）で、Go と一緒に消す。
//!
//! CLI の段（R4）の前なので、Go は CLI を流し、Rust は core を呼んで、CLI が出すのと同じ形にして比べる。
//! - `ls --json`（と `--archived`）: core の `Tree::entries` を serialize したものとバイトで比べる
//! - `graph --all`: 辺の並びを比べる（Go は `--format dot` で出して読む。表の整形は CLI の段で比べる）
//!
//! Go が要る。`go` が無ければ失敗する（Go を入れずに飛ばすなら DESK_SKIP_GO_DIFF=1）。

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use common::{Fixture, edge_tree, exception_tree, recipe_tree};
use desk_core::{Edge, Tree};
use serde_json::Value;

/// Go の CLI のバイナリ（テストの間に 1 度だけビルドする）。None なら飛ばす。
fn go_desk() -> Option<&'static Path> {
    static BIN: OnceLock<Option<PathBuf>> = OnceLock::new();
    BIN.get_or_init(|| {
        if std::env::var_os("DESK_SKIP_GO_DIFF").is_some_and(|v| v == "1") {
            eprintln!("DESK_SKIP_GO_DIFF=1 なので Go との差分テストを飛ばす");
            return None;
        }
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("desk-go");
        let status = Command::new("go")
            .args(["build", "-o"])
            .arg(&out)
            .arg("./cli/cmd/desk")
            .current_dir(repo)
            .status()
            .expect("go を動かせない（Go との差分テストには Go が要る。飛ばすなら DESK_SKIP_GO_DIFF=1）");
        assert!(status.success(), "Go の CLI をビルドできない");
        Some(out)
    })
    .as_deref()
}

/// Go の CLI をルートで動かし、標準出力を返す。
fn go(root: &Path, args: &[&str]) -> String {
    let bin = go_desk().unwrap();
    let out = Command::new(bin)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "desk {}: {}\n{}",
        args.join(" "),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// `ls --json` と同じ形（2 文字の字下げ、末尾に改行）。
fn rust_ls_json(tree: &Tree, archived: bool) -> String {
    serde_json::to_string_pretty(&tree.entries(archived)).unwrap() + "\n"
}

/// `graph --all --format dot` の行から辺を読む。
fn parse_dot(dot: &str) -> Vec<Edge> {
    dot.lines()
        .filter_map(|l| l.strip_prefix("  \""))
        .map(|l| {
            let (from, rest) = go_unquote(l);
            let rest = rest.strip_prefix(" -> \"").expect("dot の辺の形");
            let (to, rest) = go_unquote(rest);
            let rest = rest.strip_prefix(" [label=\"").expect("dot の辺の形");
            let (key, rest) = go_unquote(rest);
            assert_eq!(rest, "];");
            Edge { from, to, key }
        })
        .collect()
}

/// Go の %q で書いた文字列を、開きの " の後から読む。読んだ文字列と、閉じの " の後を返す。
fn go_unquote(s: &str) -> (String, &str) {
    let mut out = String::new();
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, &s[i + 1..]),
            '\\' => {
                let (_, e) = chars.next().unwrap();
                let mut hex = |n: usize| {
                    let h: String = (0..n).map(|_| chars.next().unwrap().1).collect();
                    u32::from_str_radix(&h, 16).unwrap()
                };
                match e {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'a' => out.push('\x07'),
                    'b' => out.push('\x08'),
                    'f' => out.push('\x0c'),
                    'v' => out.push('\x0b'),
                    'x' => out.push(char::from_u32(hex(2)).unwrap()),
                    'u' => out.push(char::from_u32(hex(4)).unwrap()),
                    'U' => out.push(char::from_u32(hex(8)).unwrap()),
                    c => out.push(c),
                }
            }
            c => out.push(c),
        }
    }
    panic!("閉じていない文字列: {s}");
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
    let got: Value = serde_json::to_value(tree.entries(false)).unwrap();
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
