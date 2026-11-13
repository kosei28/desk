//! desk の CLI。引数を読み、desk-core を呼び、出力を整える（引数・出力・使い方の文言・終了コード）。
//!
//! ツリーの形の解釈は desk-core が持ち、ここは持たない。形の正本はスキル desk（skill/SKILL.md）。

mod check;
mod graph;
mod ls;
mod read;
mod write;

use std::io::{self, Write};
use std::path::Path;

use clap::{Parser, Subcommand};
use desk_core::rules::describe;
use desk_core::{Tree, find_root};
use unicode_width::UnicodeWidthChar;

/// desk — 作業ドキュメントのツリーの形を扱う補助ツール（決まりの正本はスキル desk）
#[derive(Debug, Parser)]
#[command(
    name = "desk",
    arg_required_else_help = true,
    after_help = "種類: task artifact inbox library principle knowledge material wiki project
ルートは、作業ディレクトリから上にたどって .desk のあるディレクトリ。ツリーの外ではエラーで止まる。
書く操作はコミットしない（init の最初のコミットだけ例外）。"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// ツリーを作る（省略時は作業ディレクトリに）
    Init(write::InitArgs),
    /// いまのルートを出す
    Root,
    /// プロジェクトや文書を作る
    New(write::NewArgs),
    /// 条件で引いて一覧を出す
    Ls(ls::LsArgs),
    /// リンクをたどる
    Graph(graph::GraphArgs),
    /// 段（概要・全体・指す先まで）や節を選んで読む
    Read(read::ReadArgs),
    /// 形に合っているかを確かめる
    Check(check::CheckArgs),
    /// CLI が実装している形の一覧を出す（正本はスキル desk）
    Rules,
    /// 終わったものを archive/ に移し、リンクを付け替える
    Archive(write::ArchiveArgs),
    /// 動かす・名前を変える（リンクを付け替える）
    Mv(write::MvArgs),
}

/// CLI の入口。引数の誤りと `-h` は clap が先に扱う（main.rs）。終了コードを返す。
///
/// 書き出しの失敗は `desk: …` で 1。読む側が先に閉じた（`| head` など）ときだけ、黙って 0 で終える。
pub fn run(cli: Cli) -> u8 {
    let res = std::env::current_dir()
        .map_err(|e| fail(format!("作業ディレクトリが分からない: {e}")))
        .and_then(|cwd| {
            let out = &mut std::io::stdout().lock();
            let cwd = cwd.as_path();
            match cli.command {
                Command::Init(a) => write::init(out, cwd, a),
                Command::Root => root(out, cwd),
                Command::New(a) => write::new(out, cwd, a),
                Command::Ls(a) => ls::ls(out, cwd, a),
                Command::Graph(a) => graph::graph(out, cwd, a),
                Command::Read(a) => read::read(out, cwd, a),
                Command::Check(a) => check::check(out, cwd, a),
                Command::Rules => rules(out),
                Command::Archive(a) => write::archive(out, cwd, a),
                Command::Mv(a) => write::mv(out, cwd, a),
            }
            .and_then(|()| out.flush().map_err(Fail::from))
        });
    match res {
        Ok(()) => 0,
        Err(f) => {
            if !f.msg.is_empty() {
                note(&format!("desk: {}", f.msg));
            }
            f.code
        }
    }
}

/// 標準出力に 1 行。
fn line(out: &mut dyn Write, s: &str) -> io::Result<()> {
    writeln!(out, "{s}")
}

/// 標準出力にそのまま。
fn put(out: &mut dyn Write, s: &str) -> io::Result<()> {
    out.write_all(s.as_bytes())
}

/// 標準エラーに 1 行（知らせるだけなので、出せなくても止めない）。
fn note(s: &str) {
    let _ = writeln!(std::io::stderr(), "{s}");
}

/// 止まるときの終了コードと、`desk: ` を付けて出す文言（空なら出さない）。
#[derive(Debug)]
pub(crate) struct Fail {
    code: u8,
    msg: String,
}

impl From<io::Error> for Fail {
    /// 書き出しの失敗。読む側が先に閉じたのは失敗にしない。
    fn from(e: io::Error) -> Fail {
        if e.kind() == io::ErrorKind::BrokenPipe {
            Fail {
                code: 0,
                msg: String::new(),
            }
        } else {
            fail(format!("書き出せない: {e}"))
        }
    }
}

pub(crate) type Res = Result<(), Fail>;

/// 失敗（終了コード 1）。
fn fail(msg: impl ToString) -> Fail {
    Fail {
        code: 1,
        msg: msg.to_string(),
    }
}

/// 使い方の誤り（終了コード 2）。
fn usage(msg: impl ToString) -> Fail {
    Fail {
        code: 2,
        msg: msg.to_string(),
    }
}

/// いまのルートの絶対パスを出す。
fn root(out: &mut dyn Write, cwd: &Path) -> Res {
    let root = find_root(cwd).map_err(fail)?;
    line(out, &root.display().to_string())?;
    Ok(())
}

/// CLI が実装している形の一覧を出す。
fn rules(out: &mut dyn Write) -> Res {
    put(out, &describe())?;
    Ok(())
}

fn load(cwd: &Path) -> Result<Tree, Fail> {
    let root = find_root(cwd).map_err(fail)?;
    Tree::load(&root).map_err(fail)
}

/// カンマ区切りか繰り返しで受けた値から、前後の空白と空の値を除く。
fn list(v: Vec<String>) -> Vec<String> {
    v.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 今日（手元の時刻で YYYY-MM-DD）。
fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

/// ホームの下なら ~/ からのパスにする。
fn display_path(p: &Path) -> String {
    if let Some(home) = std::env::home_dir()
        && let Ok(rel) = p.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }
    p.display().to_string()
}

/// JSON（2 文字の字下げ、末尾に改行）。
fn write_json(out: &mut dyn Write, v: &impl serde::Serialize) -> io::Result<()> {
    let s = serde_json::to_string_pretty(v).expect("JSON にできない");
    line(out, &s)
}

/// 列をそろえて出す。列の幅は表示の幅（全角は 2）で数え、列の間は 2 文字空ける。
/// セルの中のタブは空白にし、空のセルは `-` にする。
fn table(out: &mut dyn Write, header: &[&str], rows: Vec<Vec<String>>) -> io::Result<()> {
    let mut all: Vec<Vec<String>> = vec![header.iter().map(|s| s.to_string()).collect()];
    all.extend(rows.into_iter().map(|r| {
        r.into_iter()
            .map(|c| {
                if c.is_empty() {
                    "-".into()
                } else {
                    c.replace('\t', " ")
                }
            })
            .collect()
    }));
    let mut widths: Vec<usize> = Vec::new();
    for r in &all {
        for (i, c) in r.iter().enumerate() {
            if i >= widths.len() {
                widths.push(0);
            }
            widths[i] = widths[i].max(display_width(c));
        }
    }
    for r in &all {
        let mut b = String::new();
        for (i, c) in r.iter().enumerate() {
            b += c;
            if i + 1 < r.len() {
                b += &" ".repeat(widths[i] - display_width(c) + 2);
            }
        }
        line(out, &b)?;
    }
    Ok(())
}

/// 端末に出したときの幅。文字ごとに unicode-width で数える（制御文字は 1）。
fn display_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(1)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn table_aligns_wide_characters() {
        let mut out = Vec::new();
        table(
            &mut out,
            &["title", "status"],
            vec![
                vec!["献立プランナー".into(), "active".into()],
                vec!["ab".into(), String::new()],
            ],
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "title           status\n献立プランナー  active\nab              -\n"
        );
        assert_eq!(display_width("献立a"), 5);
    }
}
