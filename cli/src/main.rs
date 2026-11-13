//! desk は作業ドキュメントのツリーの形を扱う補助ツール。決まりの正本はスキル desk（skill/SKILL.md）。

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    ExitCode::from(desk_cli::run(desk_cli::Cli::parse()))
}
