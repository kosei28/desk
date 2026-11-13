//! 差分テストで Go の CLI を動かす。Go の CLI を消す切り替え（R5）で、Go と一緒に消す。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use desk_core::Edge;

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
