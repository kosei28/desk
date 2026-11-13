//! `desk check`: 形に合っているかとずれを挙げる。`--fix` で機械的に直す。

use std::io::{self, Write};
use std::path::Path;

use desk_core::Tree;
use desk_core::check::Finding;
use desk_core::write::FixPlan;

use crate::{Fail, Res, display_path, fail, line, load, note, table, write_json};

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// プロジェクトで絞る
    #[arg(long, value_name = "名前")]
    project: Option<String>,
    /// ファイルで絞る（パスか名前。関係するファイルの分も出す）
    #[arg(long, value_name = "名前")]
    name: Option<String>,
    /// 機械的に 1 通りに決まる直しをする
    #[arg(long)]
    fix: bool,
    /// JSON で出す
    #[arg(long)]
    json: bool,
}

pub fn check(out: &mut dyn Write, cwd: &Path, a: CheckArgs) -> Res {
    let t = load(cwd)?;
    let target = match a.name.as_deref() {
        Some(n) => Some(t.resolve(n, cwd).map_err(fail)?),
        None => None,
    };
    let project = a.project.as_deref();
    let filter = |t: &Tree| -> Vec<Finding> {
        t.check()
            .into_iter()
            .filter(|x| project.is_none_or(|p| x.in_project(p)))
            .filter(|x| target.as_deref().is_none_or(|p| x.concerns(p)))
            .collect()
    };
    let mut found = filter(&t);
    let json = a.json;
    if a.fix {
        let changed = FixPlan::new(&found).apply(&t).map_err(fail)?;
        if !changed.is_empty() {
            found = filter(&Tree::load(t.root()).map_err(fail)?);
            if !json {
                line(out, "直した:")?;
                for p in &changed {
                    line(out, &format!("  {p}"))?;
                }
                line(
                    out,
                    &format!(
                        "コミットするとき: git -C {} add {}",
                        display_path(t.root()),
                        changed.join(" ")
                    ),
                )?;
                if !found.is_empty() {
                    line(out, "")?;
                }
            }
        }
    }
    if json {
        write_json(out, &found)?;
    } else {
        print_findings(out, &found, a.fix)?;
    }
    if found.is_empty() {
        Ok(())
    } else {
        // 見つけたものは出し終えているので、文言は付けない
        Err(Fail {
            code: 1,
            msg: String::new(),
        })
    }
}

fn print_findings(out: &mut dyn Write, found: &[Finding], fixed: bool) -> io::Result<()> {
    if found.is_empty() {
        note("（形に合っている）");
        return Ok(());
    }
    let rows = found
        .iter()
        .map(|f| {
            let mut p = f.path.clone();
            if f.line > 0 {
                p += &format!(":{}", f.line);
            }
            vec![p, f.category.to_string(), f.problem.clone(), f.fix.clone()]
        })
        .collect();
    table(out, &["path", "何を", "何が", "どう直すか"], rows)?;
    let fixable = found.iter().filter(|f| f.fixable).count();
    if !fixed && fixable > 0 {
        note(&format!(
            "{} 件中 {fixable} 件は --fix で直せる",
            found.len()
        ));
    }
    Ok(())
}
