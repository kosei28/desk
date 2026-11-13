//! 書く側: `init`・`new`・`archive`・`mv`・`check --fix`。feature `write` の中にだけある。
//!
//! どれも「何を作る・動かす・どの行をどう書き換えるか」の計画を作り、それを当てる。`--dry-run` は計画を出すだけ。
//! 出力の形（計画や結果の並べ方、使い方の文言、終了コード）は CLI が持つ。git は `git` のコマンドを呼ぶ。

mod fix;
mod init;
mod mv;
mod new;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use fix::FixPlan;
pub use init::{Init, init};
pub use mv::{Archive, FileEdit, Move, MovePlan, Moved};
pub use new::{NewPlan, NewRequest};

use crate::tree::clean_path;

/// 書く操作が止まったとき。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// 使い方の誤り（CLI は終了コード 2）
    Usage(String),
    /// それ以外の失敗（CLI は終了コード 1）
    Failed(String),
}

impl Error {
    fn usage(msg: impl Into<String>) -> Error {
        Error::Usage(msg.into())
    }

    fn failed(msg: impl Into<String>) -> Error {
        Error::Failed(msg.into())
    }

    /// 使い方の誤りか。
    pub fn is_usage(&self) -> bool {
        matches!(self, Error::Usage(_))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Usage(m) | Error::Failed(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Failed(e.to_string())
    }
}

/// 作業ディレクトリからのパスを、字面で整えた絶対パスにする（Go の filepath.Abs）。
pub fn absolute(cwd: &Path, p: &str) -> PathBuf {
    clean_path(&cwd.join(p))
}

// git

/// ルートが git のリポジトリか。
fn is_git_repo(root: &Path) -> bool {
    root.join(".git").exists()
}

/// ファイルが git で管理されているか。
fn git_tracked(root: &Path, rel: &str) -> bool {
    is_git_repo(root)
        && Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["ls-files", "--error-unmatch", "--", rel])
            .output()
            .is_ok_and(|o| o.status.success())
}

/// git を動かす。失敗したら git の出力をエラーにする。
fn git_run(root: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let mut msg = String::from_utf8_lossy(&out.stdout).into_owned();
    msg.push_str(&String::from_utf8_lossy(&out.stderr));
    Err(msg.trim().to_string())
}

// 空になったディレクトリ

/// .DS_Store しか無いディレクトリを消す。
fn remove_if_empty(abs: &Path) -> bool {
    let Ok(entries) = fs::read_dir(abs) else {
        return false;
    };
    for e in entries {
        let Ok(e) = e else {
            return false;
        };
        if e.file_name() != ".DS_Store" {
            return false;
        }
    }
    let _ = fs::remove_file(abs.join(".DS_Store"));
    fs::remove_dir(abs).is_ok()
}

/// 中のディレクトリが全部空なら消す。
fn remove_empty_tree(abs: &Path) -> bool {
    let Ok(entries) = fs::read_dir(abs) else {
        return false;
    };
    let mut empty = true;
    for e in entries {
        let Ok(e) = e else {
            return false;
        };
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            if !remove_empty_tree(&e.path()) {
                empty = false;
            }
        } else if e.file_name() != ".DS_Store" {
            empty = false;
        }
    }
    empty && remove_if_empty(abs)
}
