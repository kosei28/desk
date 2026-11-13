//! ビルドした desk を動かす。

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// 1 回の実行の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// git がユーザーの設定を読まないようにし、コミットの作者を決める（Go のテストの isolateGit）。
pub fn isolate_git(c: &mut Command) -> &mut Command {
    c.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
}

/// desk を cwd で、標準入力に stdin を渡して動かす。
pub fn desk_with_stdin(cwd: &Path, args: &[&str], stdin: &str) -> Out {
    let mut child = isolate_git(
        Command::new(env!("CARGO_BIN_EXE_desk"))
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
    .spawn()
    .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Out {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        code: out.status.code().unwrap(),
    }
}

/// desk を cwd で動かす（標準入力は空）。
pub fn desk(cwd: &Path, args: &[&str]) -> Out {
    desk_with_stdin(cwd, args, "")
}

/// 終了コード 0 を期待して動かし、標準出力を返す。
pub fn must(cwd: &Path, args: &[&str]) -> String {
    let o = desk(cwd, args);
    assert_eq!(
        o.code,
        0,
        "desk {}: 終了コード {}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        o.code,
        o.stdout,
        o.stderr
    );
    o.stdout
}
