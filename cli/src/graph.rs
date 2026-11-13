//! `desk graph`: リンクをたどる（tree・paths・dot）と、ツリー全体の辺（`--all`）。

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

use clap::ValueEnum;
use desk_core::graph::{Direction, Walk, edges_via};
use desk_core::{Edge, Kind, Tree};

use crate::read::{ReadOpt, kind_of, read_doc};
use crate::{Res, fail, line, list, load, table, usage};

#[derive(Debug, clap::Args)]
pub struct GraphArgs {
    /// たどり始めるファイル（パスか、1 件に決まる名前）
    #[arg(
        value_name = "パス",
        required_unless_present = "all",
        conflicts_with = "all"
    )]
    path: Option<String>,
    /// 指されているもの
    #[arg(long = "in", conflicts_with = "outbound")]
    inbound: bool,
    /// 指しているもの
    #[arg(long = "out")]
    outbound: bool,
    /// 辺の種類（frontmatter のキーか body。カンマ区切りか、繰り返して複数）
    #[arg(long, value_name = "キー", value_delimiter = ',')]
    via: Vec<String>,
    /// たどる段数（n か all）
    #[arg(long, default_value = "1", value_name = "n|all", value_parser = parse_depth)]
    depth: Depth,
    /// 出す形（既定は tree。--all では edges か dot で、既定は edges）
    #[arg(long, value_enum)]
    format: Option<Format>,
    /// たどった先を read のその段で出す
    #[arg(long, value_name = "0|1|2", value_parser = clap::value_parser!(i64).range(0..=2))]
    level: Option<i64>,
    /// ツリー全体のリンク
    #[arg(long, conflicts_with_all = ["inbound", "outbound", "depth", "level"])]
    all: bool,
}

/// 出す形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// たどった道筋を字下げで
    Tree,
    /// たどった先のパスを 1 行ずつ
    Paths,
    /// Graphviz の dot
    Dot,
    /// 辺の表（--all だけ）
    Edges,
}

/// たどる段数。None は all。
#[derive(Debug, Clone, Copy)]
struct Depth(Option<usize>);

fn parse_depth(s: &str) -> Result<Depth, String> {
    match s {
        "all" => Ok(Depth(None)),
        s => match s.parse::<usize>() {
            Ok(n) if n >= 1 => Ok(Depth(Some(n))),
            _ => Err("1 以上の数か all".into()),
        },
    }
}

pub fn graph(out: &mut dyn Write, cwd: &Path, a: GraphArgs) -> Res {
    let via = list(a.via);
    let t = load(cwd)?;
    let Some(path) = a.path else {
        let edges = edges_via(t.edges(), &via);
        match a.format.unwrap_or(Format::Edges) {
            Format::Edges => {
                let rows = edges
                    .into_iter()
                    .map(|ed| vec![ed.from, ed.key, ed.to])
                    .collect();
                table(out, &["from", "via", "to"], rows)?;
            }
            Format::Dot => dot(out, &edges)?,
            Format::Tree | Format::Paths => {
                return Err(usage("--all の --format は edges か dot"));
            }
        }
        return Ok(());
    };
    let start = t.resolve(&path, cwd).map_err(fail)?;
    if t.get(&start).is_none() {
        return Err(fail(format!("{start} はファイルでない")));
    }
    let dirs: &[Direction] = if a.inbound {
        &[Direction::In]
    } else if a.outbound {
        &[Direction::Out]
    } else {
        &[Direction::Out, Direction::In]
    };
    let w = t.walk(&start, &via, dirs, a.depth.0);
    match a.format.unwrap_or(Format::Tree) {
        Format::Tree => {
            line(out, &node_line(&t, &start))?;
            tree_lines(out, &t, &w)?;
        }
        Format::Paths => {
            for p in &w.order {
                line(out, p)?;
            }
        }
        Format::Dot => dot(out, &w.edges)?,
        Format::Edges => return Err(usage("--format edges は --all のときだけ")),
    }
    if let Some(level) = a.level
        && !w.order.is_empty()
    {
        line(out, "")?;
        for (i, p) in w.order.iter().enumerate() {
            if i > 0 && level > 0 {
                line(out, "")?;
            }
            let d = t.get(p).expect("たどった先はファイル");
            read_doc(out, &t, d, &ReadOpt::level(level), true)?;
        }
    }
    Ok(())
}

fn tree_lines(out: &mut dyn Write, t: &Tree, w: &Walk) -> io::Result<()> {
    for s in &w.steps {
        let indent = "  ".repeat(s.depth);
        let arrow = match s.dir {
            Direction::Out => "→",
            Direction::In => "←",
        };
        if s.seen {
            line(
                out,
                &format!("{indent}{arrow} {}  {}（既出）", s.key, s.node),
            )?;
        } else {
            line(
                out,
                &format!("{indent}{arrow} {}  {}", s.key, node_line(t, &s.node)),
            )?;
        }
    }
    Ok(())
}

/// パス、種類（タスクなら status も）、description（無ければ見出し）。
fn node_line(t: &Tree, p: &str) -> String {
    let Some(d) = t.get(p) else {
        return p.to_string();
    };
    let mut s = format!("{p}  [{}", kind_of(d));
    let st = d.status();
    if d.info.kind == Some(Kind::Task) && !st.is_empty() {
        s += &format!(" · {st}");
    }
    s.push(']');
    let desc = d.description();
    if !desc.is_empty() {
        s += &format!("  {desc}");
    } else if !d.title().is_empty() {
        s += &format!("  {}", d.title());
    }
    s
}

fn dot(out: &mut dyn Write, edges: &[Edge]) -> io::Result<()> {
    line(out, "digraph desk {")?;
    line(out, "  rankdir=LR;")?;
    line(out, "  node [shape=box, fontsize=10];")?;
    let mut seen = HashSet::new();
    for ed in edges {
        if seen.insert(ed) {
            line(
                out,
                &format!(
                    "  {} -> {} [label={}];",
                    dot_id(&ed.from),
                    dot_id(&ed.to),
                    dot_id(&ed.key)
                ),
            )?;
        }
    }
    line(out, "}")
}

/// DOT の引用された ID（`"` で囲み、中の `"` と `\` をエスケープする）。
fn dot_id(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::dot_id;

    #[test]
    fn dot_id_escapes_quote_and_backslash_only() {
        assert_eq!(dot_id("a.md"), r#""a.md""#);
        assert_eq!(dot_id(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(dot_id("献立\u{a0}.md"), "\"献立\u{a0}.md\"");
    }
}
