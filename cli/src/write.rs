//! 書く操作: `desk init`・`new`・`archive`・`mv`。どれもコミットはしない（init の最初のコミットだけ例外）。

use std::io::Write;
use std::path::Path;

use desk_core::Tree;
use desk_core::write::{Error, MovePlan, NewRequest, absolute, init as core_init};

use crate::{Fail, Res, display_path, fail, line, load, today, usage};

/// core の書く側のエラーを終了コードに分ける。
fn err(e: Error) -> Fail {
    if e.is_usage() { usage(e) } else { fail(e) }
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// ツリーのルートにするディレクトリ（省略時は作業ディレクトリ）
    #[arg(value_name = "パス")]
    path: Option<String>,
}

pub fn init(out: &mut dyn Write, cwd: &Path, a: InitArgs) -> Res {
    let root = absolute(cwd, a.path.as_deref().unwrap_or("."));
    let r = core_init(&root).map_err(err)?;
    if r.already {
        line(
            out,
            &format!("{} はもうツリーのルート（作るものは無い）", root.display()),
        )?;
        return Ok(());
    }
    line(out, &format!("{} を作った:", root.display()))?;
    for c in &r.created {
        line(out, &format!("  {c}"))?;
    }
    if r.committed {
        line(out, "最初のコミットを作った")?;
    } else if !r.written.is_empty() {
        line(
            out,
            &format!(
                "コミットはしていない: git -C {} add {}",
                display_path(&root),
                r.written.join(" ")
            ),
        )?;
    }
    line(out, "desk はこのディレクトリかその下で実行する")?;
    Ok(())
}

#[derive(Debug, clap::Args)]
#[command(override_usage = "desk new project <名前>\n       desk new <種類> <slug> [OPTIONS]")]
pub struct NewArgs {
    /// 種類（project か、task・artifact などファイルの種類）
    #[arg(value_name = "種類")]
    kind: String,
    /// 名前（project なら名前、ほかは slug）
    #[arg(value_name = "slug")]
    slug: String,
    /// プロジェクト
    #[arg(long, value_name = "名前")]
    project: Option<String>,
    /// 親タスク（task）か、文書のタスク（artifact の tasks）
    #[arg(long, value_name = "タスク")]
    parent: Option<String>,
    /// task に needs_human: true を書く（人の判断か作業が要る）
    #[arg(long)]
    needs_human: bool,
    /// 見出し（省略は slug）
    #[arg(long, value_name = "見出し")]
    title: Option<String>,
}

pub fn new(out: &mut dyn Write, cwd: &Path, a: NewArgs) -> Res {
    let t = load(cwd)?;
    let today = today().to_string();
    let req = NewRequest {
        kind: &a.kind,
        slug: &a.slug,
        project: a.project.as_deref(),
        parent: a.parent.as_deref(),
        needs_human: a.needs_human,
        title: a.title.as_deref(),
        today: &today,
    };
    let plan = t.plan_new(&req, cwd).map_err(err)?;
    let p = plan.apply(&t).map_err(err)?;
    line(out, &p.display().to_string())?;
    Ok(())
}

#[derive(Debug, clap::Args)]
pub struct ArchiveArgs {
    /// 動かすファイル（タスクと、一緒に動かす文書）
    #[arg(value_name = "パス", required_unless_present = "project")]
    paths: Vec<String>,
    /// タスクに書く status（省略は done、done・dropped ならそのまま）
    #[arg(long, value_parser = ["done", "dropped"])]
    status: Option<String>,
    /// プロジェクトごとルートの archive/ へ
    #[arg(long, value_name = "名前", conflicts_with_all = ["paths", "status"])]
    project: Option<String>,
    /// 何を動かし、どこを書き換えるかだけ出す
    #[arg(long)]
    dry_run: bool,
}

pub fn archive(out: &mut dyn Write, cwd: &Path, a: ArchiveArgs) -> Res {
    let t = load(cwd)?;
    if let Some(name) = &a.project {
        let plan = t.plan_archive_project(name).map_err(err)?;
        return perform(out, &t, &plan, a.dry_run);
    }
    let srcs: Vec<&str> = a.paths.iter().map(String::as_str).collect();
    let a_status = a.status.as_deref();
    let plan = t.plan_archive(&srcs, a_status, cwd);
    for p in &plan.already {
        line(out, &format!("既に archive/ にある: {p}"))?;
    }
    match plan.plan.map_err(err)? {
        Some(p) => perform(out, &t, &p, a.dry_run),
        None => Ok(()),
    }
}

#[derive(Debug, clap::Args)]
#[command(
    override_usage = "desk mv <元>... <先のディレクトリ> [--dry-run]\n       desk mv <元のファイル> <新しいパス.md> [--dry-run]   # 名前の変更"
)]
pub struct MvArgs {
    /// 元（1 つ以上）と、最後に先
    #[arg(value_name = "パス", num_args = 2.., required = true)]
    paths: Vec<String>,
    /// 何を動かし、どこを書き換えるかだけ出す
    #[arg(long)]
    dry_run: bool,
}

pub fn mv(out: &mut dyn Write, cwd: &Path, a: MvArgs) -> Res {
    let (dest, srcs) = a.paths.split_last().expect("clap が 2 つ以上を求める");
    let t = load(cwd)?;
    let srcs: Vec<&str> = srcs.iter().map(String::as_str).collect();
    let plan = t.plan_mv(&srcs, dest, cwd).map_err(err)?;
    perform(out, &t, &plan, a.dry_run)
}

/// 計画を出すか（--dry-run）、当てて結果を出す。
fn perform(out: &mut dyn Write, t: &Tree, plan: &MovePlan, dry: bool) -> Res {
    if dry {
        line(out, "動かす:")?;
        for m in &plan.moves {
            line(out, &format!("  {} → {}", m.from, m.to))?;
        }
        if !plan.edits.is_empty() {
            line(out, "書き換える:")?;
        }
        for (p, fe) in &plan.edits {
            for ed in &fe.edits {
                line(
                    out,
                    &format!("  {p}:{}  {} → {}", ed.line + 1, ed.old, ed.new),
                )?;
            }
            for ins in &fe.inserts {
                line(out, &format!("  {p}  （frontmatter に足す）{ins}"))?;
            }
        }
        return Ok(());
    }
    let res = plan.apply(t).map_err(err)?;
    line(out, "動かした:")?;
    for m in &res.moved {
        line(out, &format!("  {} → {}", m.from, m.to))?;
    }
    if !res.rewritten.is_empty() {
        line(out, "書き換えた（リンク・status）:")?;
        for p in &res.rewritten {
            line(out, &format!("  {p}"))?;
        }
    }
    line(
        out,
        &format!(
            "コミットするとき: git -C {} add {}",
            display_path(t.root()),
            res.add_paths().join(" ")
        ),
    )?;
    Ok(())
}
