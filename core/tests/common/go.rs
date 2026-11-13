//! 差分テストで Go の CLI を動かす。Go の CLI を消す切り替え（R5）で、Go と一緒に消す。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use desk_core::{Edge, Tree};
use serde::{Deserialize, Serialize};

use super::cli::Out;

/// Go の CLI のバイナリ（テストの間に 1 度だけビルドする）。None なら飛ばす。
pub fn go_desk() -> Option<&'static Path> {
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

/// Go の CLI を cwd で動かす。
pub fn go_run(cwd: &Path, args: &[&str]) -> Out {
    let bin = go_desk().unwrap();
    let out = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    Out {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        code: out.status.code().unwrap(),
    }
}

/// Go の CLI を cwd で動かし、標準出力を返す（失敗したら止める）。
pub fn go(cwd: &Path, args: &[&str]) -> String {
    let out = go_run(cwd, args);
    assert!(
        out.code == 0,
        "desk {}: {}\n{}",
        args.join(" "),
        out.code,
        out.stderr
    );
    out.stdout
}

/// `graph --all --format dot` の行から辺を読む。
pub fn parse_dot(dot: &str) -> Vec<Edge> {
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
pub fn go_unquote(s: &str) -> (String, &str) {
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

/// Go の `check --json` の 1 件（項目の順も Go のまま読み書きする）。
#[derive(Debug, Serialize, Deserialize)]
pub struct GoFinding {
    pub path: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub line: usize,
    pub category: String,
    pub problem: String,
    pub fix: String,
    pub fixable: bool,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Go と変えたところ: frontmatter の値の行。Go は YAML のノードの行（frontmatter の中で 1 始まり、つまりファイルの
/// 0 始まりの行）を 1 始まりの行として出し、1 行上を指していた。Rust は値のある行を 1 始まりで出す。
/// 値の行を出すのは、キーがあるときの「X が空」「X の値 … は決まりの外」「X の値 … が日付でない」。
pub fn fix_go_fm_lines(root: &Path, want: &mut Out) -> usize {
    if want.stdout.is_empty() {
        return 0;
    }
    let tree = Tree::load(root).unwrap();
    let mut found: Vec<GoFinding> = serde_json::from_str(&want.stdout).unwrap();
    let mut n = 0;
    for x in &mut found {
        if x.category != "frontmatter" {
            continue;
        }
        let key = x.problem.split(' ').next().unwrap();
        let has_key = tree
            .get(&x.path)
            .and_then(|d| d.frontmatter.as_ref())
            .is_some_and(|fm| fm.get(key).is_some());
        if has_key {
            x.line += 1;
            n += 1;
        }
    }
    want.stdout = serde_json::to_string_pretty(&found).unwrap() + "\n";
    n
}

/// Go と変えたところ: YAML を読めないときの文言（Go は yaml.v3、Rust は saphyr の文）。行と、ほかの項目は同じ。
pub fn fix_go_yaml_errors(root: &Path, want: &mut Out, got: &Out) -> usize {
    if want.stdout.is_empty() {
        return 0;
    }
    let tree = Tree::load(root).unwrap();
    let mut found: Vec<GoFinding> = serde_json::from_str(&want.stdout).unwrap();
    let ours: Vec<GoFinding> = serde_json::from_str(&got.stdout).unwrap();
    let mut n = 0;
    for x in &mut found {
        let Some(err) = tree.get(&x.path).and_then(|d| d.fm_error.clone()) else {
            continue;
        };
        if x.category == "frontmatter" && x.line == 1 && x.fix == "手で直す" {
            assert!(
                ours.iter().any(|o| o.path == x.path && o.problem == err),
                "{} の YAML のエラーが Rust に無い",
                x.path
            );
            x.problem = err;
            n += 1;
        }
    }
    want.stdout = serde_json::to_string_pretty(&found).unwrap() + "\n";
    n
}
