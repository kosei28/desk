//! 引く: `desk ls` の条件で絞る。

use relative_path::RelativePath;

use crate::doc::one_line;
use crate::link;
use crate::rules::{Kind, link_keys};
use crate::tree::{Entry, Tree};

/// `--where` の演算子。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `=`: 値のどれかが当たる（パスなら名前だけでも当たる）。`[]` か空なら値が無い
    Eq,
    /// `!=`: `=` の逆
    Ne,
    /// `~`: 値のどれかが含む（`a,b` はどれか）
    Contains,
    /// `<`: 数なら数で、ほかは文字列で比べる
    Lt,
    /// `>`
    Gt,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Ne => "!=",
            Op::Contains => "~",
            Op::Lt => "<",
            Op::Gt => ">",
        }
    }
}

/// `--where` の条件 1 つ（`<キー><演算子><値>`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Where {
    pub key: String,
    pub op: Op,
    pub val: String,
}

/// `--where` を読めないとき。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhereError {
    NoKey(String),
    NoOp(String),
}

impl std::fmt::Display for WhereError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhereError::NoKey(s) => {
                write!(f, "--where {s:?}: キーが無い（例: status=inprogress）")
            }
            WhereError::NoOp(s) => write!(f, "--where {s:?}: 演算子は = != ~ < >"),
        }
    }
}

impl std::error::Error for WhereError {}

impl Where {
    /// `<キー><演算子><値>` を読む。キーは英数字・`_`・`-`・`.`。
    pub fn parse(s: &str) -> Result<Where, WhereError> {
        let i = s
            .bytes()
            .take_while(|&c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
            .count();
        if i == 0 {
            return Err(WhereError::NoKey(s.to_string()));
        }
        let rest = &s[i..];
        for op in [Op::Ne, Op::Eq, Op::Contains, Op::Lt, Op::Gt] {
            if let Some(val) = rest.strip_prefix(op.as_str()) {
                return Ok(Where {
                    key: s[..i].to_string(),
                    op,
                    val: val.to_string(),
                });
            }
        }
        Err(WhereError::NoOp(s.to_string()))
    }

    /// その 1 件に当たるか。
    pub fn matches(&self, tree: &Tree, e: &Entry<'_>) -> bool {
        let vals = e.field_values(tree, &self.key);
        match self.op {
            Op::Eq | Op::Ne => {
                let ok = if self.val.is_empty() || self.val == "[]" {
                    vals.is_empty()
                } else {
                    vals.iter()
                        .any(|v| value_matches(e, &self.key, v, &self.val))
                };
                ok == (self.op == Op::Eq)
            }
            Op::Contains => {
                let alts = split_comma(&self.val);
                vals.iter()
                    .any(|v| alts.iter().any(|a| v.contains(a.as_str())))
            }
            Op::Lt | Op::Gt => vals.iter().any(|v| compare(v, &self.val, self.op)),
        }
    }
}

/// 値 v（パスかもしれない）が val に当たるか。パスなら名前だけでも当たる。
fn value_matches(e: &Entry<'_>, key: &str, v: &str, val: &str) -> bool {
    if v == val {
        return true;
    }
    let p = v.split_once('#').map_or(v, |(p, _)| p);
    let b = RelativePath::new(p).file_name().unwrap_or(p);
    if p == val || b == val || b.strip_suffix(".md").unwrap_or(b) == val {
        return true;
    }
    if link_keys(e.info.kind).contains(&key) && !p.starts_with('/') && !p.starts_with('~') {
        let r = link::target(&e.info.path, p);
        if r == val || r == val.strip_suffix('/').unwrap_or(val) {
            return true;
        }
    }
    false
}

/// 両方が数（有限の 10 進）なら数で、ほかは文字列で比べる。
fn compare(a: &str, b: &str, op: Op) -> bool {
    let num = |s: &str| s.parse::<f64>().ok().filter(|x| x.is_finite());
    match (num(a), num(b)) {
        (Some(x), Some(y)) if op == Op::Lt => x < y,
        (Some(x), Some(y)) => x > y,
        _ if op == Op::Lt => a < b,
        _ => a > b,
    }
}

/// カンマで分け、空白を除き、空のものを捨てる。
fn split_comma(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

impl Entry<'_> {
    /// キーの値（frontmatter か、name・path・project・kind・title・now・modified）。
    pub fn field_values(&self, tree: &Tree, key: &str) -> Vec<String> {
        let one = |s: &str| {
            if s.is_empty() {
                Vec::new()
            } else {
                vec![s.to_string()]
            }
        };
        match key {
            "name" => one(self.name()),
            "path" => one(&self.info.path),
            "project" => one(&self.info.project),
            "kind" => one(self.info.kind.map_or("", Kind::as_str)),
            "title" => one(self.title()),
            "now" => one(&self.now()),
            "modified" => one(self.modified(tree)),
            _ => self
                .doc
                .filter(|d| d.has_key(key))
                .map(|d| d.list(key))
                .unwrap_or_default(),
        }
    }

    /// キーの値を 1 行の文字列にする（配列はカンマでつなぐ）。
    pub fn field_string(&self, tree: &Tree, key: &str) -> String {
        if key == "description" {
            return self.doc.map(|d| d.description()).unwrap_or_default();
        }
        one_line(&self.field_values(tree, key).join(", "))
    }
}

/// `desk ls` の条件。
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// 空なら全部の種類
    pub kinds: Vec<Kind>,
    /// プロジェクトで絞る（原則とナレッジはルートのものも入る）
    pub project: Option<String>,
    /// archive/ の中も入れる
    pub archived: bool,
    /// ファイル名（.md は省略できる）かプロジェクトの名前
    pub name: Option<String>,
    /// 全部に当たるもの（AND）
    pub wheres: Vec<Where>,
    pub modified: Option<Modified>,
}

/// 最終コミットの日付で絞る。コミットされていないものは today に変えたとして比べる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modified {
    /// 今日（YYYY-MM-DD）
    pub today: String,
    /// この日より前（`--modified-before N` なら today の N 日前）
    pub before: Option<String>,
    /// この日以降（`--modified-after N` なら today の N 日前）
    pub after: Option<String>,
}

impl Tree {
    /// 条件に当たるもの（`desk ls` の並び）。
    pub fn ls(&self, q: &Query) -> Vec<Entry<'_>> {
        self.entries(&q.kinds, q.project.as_deref(), q.archived)
            .into_iter()
            .filter(|e| {
                q.name.as_deref().is_none_or(|n| {
                    let name = e.info.name();
                    name == n
                        || name.strip_suffix(".md") == Some(n)
                        || e.info.kind == Some(Kind::Project) && e.info.project == n
                })
            })
            .filter(|e| q.wheres.iter().all(|w| w.matches(self, e)))
            .filter(|e| {
                let Some(m) = &q.modified else { return true };
                let mut d = e.modified(self);
                if d.is_empty() {
                    d = &m.today;
                }
                m.before.as_deref().is_none_or(|b| d < b)
                    && m.after.as_deref().is_none_or(|a| d >= a)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_where() {
        assert_eq!(
            Where::parse("status!=done"),
            Ok(Where {
                key: "status".into(),
                op: Op::Ne,
                val: "done".into()
            })
        );
        assert_eq!(Where::parse("=x"), Err(WhereError::NoKey("=x".into())));
        assert_eq!(Where::parse("a?b"), Err(WhereError::NoOp("a?b".into())));
        assert_eq!(Where::parse("sources=[]").unwrap().val, "[]");
    }

    #[test]
    fn compare_numbers_and_strings() {
        assert!(compare("9", "10", Op::Lt));
        assert!(compare("2026-11-02", "2026-11-10", Op::Lt));
        assert!(compare("b", "a", Op::Gt));
        assert!(compare("1e3", "999", Op::Gt));
        assert!(!compare("0x10", "9", Op::Gt));
        // inf・nan は数にしない（文字列で比べる）
        assert!(!compare("-inf", "-5", Op::Lt));
        assert!(compare("nan", "5", Op::Gt));
    }
}
