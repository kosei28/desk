//! `desk ls`: 条件で引き、表・`--json`・`--paths`・`--level` で出す。

use std::io::Write;
use std::path::Path;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use desk_core::Kind;
use desk_core::query::{Modified, Query, Where};
use desk_core::rules::ALL_KINDS;
use jiff::civil::Date;
use serde_json::Value;

use crate::read::{ReadOpt, read_doc};
use crate::{Res, line, list, load, note, table, today, write_json};

#[derive(Debug, clap::Args)]
#[command(group = clap::ArgGroup::new("output").args(["json", "paths", "level"]))]
pub struct LsArgs {
    /// 種類（カンマ区切りか、並べて複数）
    #[arg(value_name = "種類", value_delimiter = ',', value_parser = kind_parser())]
    kinds: Vec<Kind>,
    /// ファイル名で引く
    #[arg(long, value_name = "名前")]
    name: Option<String>,
    /// プロジェクトで絞る（原則とナレッジはルートのものも入る）
    #[arg(long, value_name = "名前")]
    project: Option<String>,
    /// archive/ の中も入れる
    #[arg(long)]
    archived: bool,
    /// <キー><演算子><値>（= != ~ < >。複数は AND）
    #[arg(long = "where", value_name = "条件", value_parser = parse_where)]
    wheres: Vec<Where>,
    /// 最終コミットがこの日より前（N 日前か YYYY-MM-DD）
    #[arg(long, value_name = "日数|日付", value_parser = parse_day)]
    modified_before: Option<Day>,
    /// 最終コミットがこの日以降（N 日前か YYYY-MM-DD）
    #[arg(long, value_name = "日数|日付", value_parser = parse_day)]
    modified_after: Option<Day>,
    /// 列（name path project modified kind title now と frontmatter のキー。カンマ区切りか、繰り返して複数）
    #[arg(long, value_name = "列", value_delimiter = ',', conflicts_with_all = ["paths", "level"])]
    fields: Vec<String>,
    /// JSON で出す
    #[arg(long)]
    json: bool,
    /// パスだけ出す
    #[arg(long)]
    paths: bool,
    /// 当たったものを read のその段で出す
    #[arg(long, value_name = "0|1|2", value_parser = clap::value_parser!(i64).range(0..=2))]
    level: Option<i64>,
}

/// 種類の名前を Kind にする（知らない名前は clap が選べるものを添えて断る）。
fn kind_parser() -> impl TypedValueParser<Value = Kind> {
    PossibleValuesParser::new(ALL_KINDS.iter().map(|k| k.as_str()))
        .map(|s| Kind::from_name(&s).expect("選べる種類は Kind になる"))
}

fn parse_where(s: &str) -> Result<Where, String> {
    Where::parse(s).map_err(|e| e.to_string())
}

/// `--modified-before`・`--modified-after` の日。
#[derive(Debug, Clone, Copy)]
enum Day {
    /// 今日から N 日前
    Ago(i32),
    /// その日
    Date(Date),
}

fn parse_day(s: &str) -> Result<Day, String> {
    if let Ok(n) = s.parse::<u16>() {
        return Ok(Day::Ago(i32::from(n)));
    }
    s.parse::<Date>()
        .map(Day::Date)
        .map_err(|_| "日数（0 以上の数）か日付（YYYY-MM-DD）".into())
}

impl Day {
    fn date(self, today: Date) -> String {
        match self {
            Day::Ago(n) => today
                .checked_sub(jiff::Span::new().days(n))
                .expect("日数は u16 に収まる")
                .to_string(),
            Day::Date(d) => d.to_string(),
        }
    }
}

pub fn ls(out: &mut dyn Write, cwd: &Path, a: LsArgs) -> Res {
    let q = Query {
        kinds: a.kinds,
        wheres: a.wheres,
        name: a.name,
        project: a.project,
        archived: a.archived,
        modified: (a.modified_before.is_some() || a.modified_after.is_some()).then(|| {
            let today = today();
            Modified {
                before: a.modified_before.map(|d| d.date(today)),
                after: a.modified_after.map(|d| d.date(today)),
                today: today.to_string(),
            }
        }),
    };
    let t = load(cwd)?;
    let hits = t.ls(&q);
    if a.paths {
        for h in &hits {
            line(out, &h.info.path)?;
        }
    } else if let Some(level) = a.level {
        for (i, h) in hits.iter().enumerate() {
            if i > 0 && level > 0 {
                line(out, "")?;
            }
            if h.info.kind == Some(Kind::Project) {
                // プロジェクトは README.md を読む（無ければ名前だけ）
                match t.get(&format!("{}/README.md", h.info.path)) {
                    Some(r) => read_doc(out, &t, r, &ReadOpt::level(level), true)?,
                    None => line(out, &format!("{}  [project]", h.info.path))?,
                }
                continue;
            }
            let d = h.doc.expect("プロジェクトでないものはファイル");
            read_doc(out, &t, d, &ReadOpt::level(level), true)?;
        }
    } else if a.json {
        let cols = list(a.fields);
        let rows: Vec<Value> = hits
            .iter()
            .map(|h| {
                let mut v = serde_json::to_value(h).expect("JSON にできない");
                let m = v.as_object_mut().expect("1 件は JSON のオブジェクト");
                for c in &cols {
                    if !m.contains_key(c) {
                        m.insert(c.clone(), Value::String(h.field_string(&t, c)));
                    }
                }
                v
            })
            .collect();
        write_json(out, &rows)?;
    } else {
        let mut cols = list(a.fields);
        if cols.is_empty() {
            cols = default_fields(&q.kinds)
                .iter()
                .map(|s| s.to_string())
                .collect();
        }
        if hits.is_empty() {
            note("（当たるものは無い）");
            return Ok(());
        }
        let rows = hits
            .iter()
            .map(|h| cols.iter().map(|c| h.field_string(&t, c)).collect())
            .collect();
        let header: Vec<&str> = cols.iter().map(String::as_str).collect();
        table(out, &header, rows)?;
    }
    Ok(())
}

/// 表の既定の列（種類を 1 つに絞ったときは、その種類で見たい列）。
fn default_fields(kinds: &[Kind]) -> &'static [&'static str] {
    let [kind] = kinds else {
        return &["path", "kind", "title"];
    };
    match kind {
        Kind::Task => &["path", "status", "needs_human", "now"],
        Kind::Artifact => &["path", "tasks", "title"],
        Kind::Wiki | Kind::Principle | Kind::Knowledge => &["path", "description"],
        Kind::Material => &["path", "knowledge", "title"],
        Kind::Library => &["path", "tags", "title"],
        Kind::Project => &["name", "title"],
        _ => &["path", "title"],
    }
}
