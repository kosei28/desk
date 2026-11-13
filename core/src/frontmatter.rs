//! frontmatter。値は YAML として型を解かず、書いてあるとおりの文字列か文字列の配列として扱う。
//! 書き換えは YAML を書き直さず行の置き換えでやるので、値ごとに行と位置を持つ。

use saphyr_parser::{Event, Parser, ScalarStyle, ScanError, Span};
use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};

/// frontmatter の値 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Value {
    /// 書いていない（`key:`、`~`、`null`）か、文字列にも文字列の配列にもならないもの（入れ子の対応など）
    Null,
    Str(String),
    /// 配列。要素のうち null と、文字列でないもの（入れ子）は入れない
    List(Vec<String>),
}

/// 値の中のスカラー 1 つ（文字列と、書いてある位置）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar {
    pub value: String,
    /// ファイルの行（0 始まり）。frontmatter の 1 行目はファイルの 1 行目
    pub line: usize,
    /// 行の中の位置（0 始まりの文字数）。引用符で囲んだ値は引用符の位置
    pub column: usize,
    pub quoted: bool,
}

/// キーと値 1 組。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: String,
    pub value: Value,
    /// 値の始まりの行（ファイルの行、0 始まり）。値を書いていなければキーの行
    pub line: usize,
    /// 値の中のスカラー（null を除く）。配列なら要素ごと
    pub scalars: Vec<Scalar>,
}

/// frontmatter 全体。キーの順は書いてあるとおりで、同じキーが 2 度あればどちらも持つ。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frontmatter {
    pub fields: Vec<Field>,
}

impl Frontmatter {
    /// キーの値（同じキーが 2 度あれば先のもの）。
    pub fn get(&self, key: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// キーを順に返す。
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|f| f.key.as_str())
    }

    /// 書いてあるとおりの順のキーと値。同じキーが 2 度あれば、get と同じく最初のものだけ。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &Value)> {
        let fs = &self.fields;
        fs.iter()
            .enumerate()
            .filter(|(i, f)| !fs[..*i].iter().any(|g| g.key == f.key))
            .map(|(_, f)| (f.key.as_str(), &f.value))
    }
}

/// 書いてあるとおりの順で、`[{"key": …, "value": …}, …]` の形で出す。
impl Serialize for Frontmatter {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.fields.len()))?;
        for f in &self.fields {
            seq.serialize_element(&FieldOut {
                key: &f.key,
                value: &f.value,
            })?;
        }
        seq.end()
    }
}

#[derive(Serialize)]
struct FieldOut<'a> {
    key: &'a str,
    value: &'a Value,
}

/// 書いてあるとおりの順の対応として出す（`ls --json` の frontmatter）。
pub(crate) struct FrontmatterMap<'a>(pub Option<&'a Frontmatter>);

impl Serialize for FrontmatterMap<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        for (k, v) in self.0.into_iter().flat_map(Frontmatter::entries) {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

/// frontmatter が読めないとき。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmError {
    /// YAML として読めない
    Yaml(String),
    /// 「キー: 値」の形でない
    NotMapping,
}

impl std::fmt::Display for FmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FmError::Yaml(e) => write!(f, "frontmatter が YAML として読めない: {e}"),
            FmError::NotMapping => f.write_str("frontmatter が「キー: 値」の形でない"),
        }
    }
}

/// YAML のノード（読むのに要る分だけ）。
enum Node {
    Scalar {
        scalar: Scalar,
        null: bool,
    },
    Seq {
        line: usize,
        items: Vec<Node>,
    },
    Map {
        line: usize,
        pairs: Vec<(Node, Node)>,
    },
    Alias {
        line: usize,
    },
}

impl Node {
    /// 始まりの行（ファイルの行、0 始まり）。
    fn line(&self) -> usize {
        match self {
            Node::Scalar { scalar, .. } => scalar.line,
            Node::Seq { line, .. } | Node::Map { line, .. } | Node::Alias { line } => *line,
        }
    }
}

/// `---` と `---` の間の YAML（ファイルの 2 行目から）を読む。
pub fn parse(text: &str) -> Result<Frontmatter, FmError> {
    let mut p = Reader {
        parser: Parser::new_from_str(text),
        len: text.chars().count(),
        ends_with_newline: text.ends_with('\n'),
    };
    let root = p.document().map_err(|e| FmError::Yaml(e.to_string()))?;
    let Some(root) = root else {
        return Ok(Frontmatter::default());
    };
    let pairs = match root {
        Node::Map { pairs, .. } => pairs,
        Node::Scalar { null: true, .. } => return Ok(Frontmatter::default()),
        _ => return Err(FmError::NotMapping),
    };
    let fields = pairs
        .into_iter()
        .map(|(k, v)| {
            let key = match k {
                Node::Scalar { scalar, .. } => scalar.value,
                _ => String::new(),
            };
            let line = v.line();
            let (value, scalars) = match v {
                Node::Scalar { null: true, .. } => (Value::Null, Vec::new()),
                Node::Scalar { scalar, .. } => (Value::Str(scalar.value.clone()), vec![scalar]),
                Node::Seq { items, .. } => {
                    let scalars: Vec<Scalar> = items
                        .into_iter()
                        .filter_map(|n| match n {
                            Node::Scalar {
                                scalar,
                                null: false,
                            } => Some(scalar),
                            _ => None,
                        })
                        .collect();
                    (
                        Value::List(scalars.iter().map(|s| s.value.clone()).collect()),
                        scalars,
                    )
                }
                Node::Map { .. } | Node::Alias { .. } => (Value::Null, Vec::new()),
            };
            Field {
                key,
                value,
                line,
                scalars,
            }
        })
        .collect();
    Ok(Frontmatter { fields })
}

struct Reader<'a> {
    parser: Parser<'a, saphyr_parser::StrInput<'a>>,
    /// 入力の文字数
    len: usize,
    ends_with_newline: bool,
}

impl<'a> Reader<'a> {
    fn next(&mut self) -> Result<(Event<'a>, Span), ScanError> {
        match self.parser.next_event() {
            Some(r) => r,
            None => Ok((Event::StreamEnd, Span::default())),
        }
    }

    /// 最初の文書のいちばん上のノード。文書が無ければ None。
    fn document(&mut self) -> Result<Option<Node>, ScanError> {
        loop {
            match self.next()?.0 {
                Event::StreamStart | Event::Nothing => {}
                Event::DocumentStart(_) => break,
                _ => return Ok(None),
            }
        }
        let (ev, span) = self.next()?;
        match ev {
            Event::DocumentEnd | Event::StreamEnd => Ok(None),
            ev => Ok(Some(self.node(ev, span)?)),
        }
    }

    fn node(&mut self, ev: Event<'a>, span: Span) -> Result<Node, ScanError> {
        // span の行は 1 始まり。YAML の 1 行目はファイルの 2 行目（0 始まりで 1）なので、そのままファイルの行になる
        let line = span.start.line();
        Ok(match ev {
            Event::Scalar(mut value, style, _, tag) => {
                // 改行で終わらない入力の終わりまで続くブロック（| や >）は、終わりに改行を足さない
                // （yaml.v3 と同じ。frontmatter は閉じる --- の前で切るので、最後の値がこれに当たる）
                if matches!(style, ScalarStyle::Literal | ScalarStyle::Folded)
                    && !self.ends_with_newline
                    && span.end.index() >= self.len
                    && value.ends_with('\n')
                {
                    value.to_mut().pop();
                }
                let null = match &tag {
                    Some(t) => t.is_yaml_core_schema() && t.suffix == "null",
                    None => {
                        style == ScalarStyle::Plain
                            && matches!(&*value, "" | "~" | "null" | "Null" | "NULL")
                    }
                };
                Node::Scalar {
                    scalar: Scalar {
                        value: value.into_owned(),
                        line,
                        column: span.start.col(),
                        quoted: matches!(
                            style,
                            ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted
                        ),
                    },
                    null,
                }
            }
            Event::SequenceStart(..) => {
                let mut items = Vec::new();
                loop {
                    let (ev, span) = self.next()?;
                    if matches!(ev, Event::SequenceEnd) {
                        break;
                    }
                    items.push(self.node(ev, span)?);
                }
                Node::Seq { line, items }
            }
            Event::MappingStart(..) => {
                let mut pairs = Vec::new();
                loop {
                    let (ev, span) = self.next()?;
                    if matches!(ev, Event::MappingEnd) {
                        break;
                    }
                    let k = self.node(ev, span)?;
                    let (ev, span) = self.next()?;
                    let v = self.node(ev, span)?;
                    pairs.push((k, v));
                }
                Node::Map { line, pairs }
            }
            Event::Alias(_) => Node::Alias { line },
            // 文書の途中で終わる（パーサーが先にエラーを返すので、ふつうは来ない）
            _ => Node::Alias { line },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_as_written() {
        let fm = parse("status: todo\nparent:\nx: ~\nn: 1.50\ndeps: [a.md, \"b.md\", 1, null]\nt: !!str\nu: !!null x\nm: {a: b}\n").unwrap();
        let v = |k: &str| fm.get(k).unwrap().value.clone();
        assert_eq!(v("status"), Value::Str("todo".into()));
        assert_eq!(v("parent"), Value::Null);
        assert_eq!(v("x"), Value::Null);
        assert_eq!(v("n"), Value::Str("1.50".into()));
        assert_eq!(
            v("deps"),
            Value::List(vec!["a.md".into(), "b.md".into(), "1".into()])
        );
        assert_eq!(v("t"), Value::Str(String::new()));
        assert_eq!(v("u"), Value::Null);
        assert_eq!(v("m"), Value::Null);
        let deps = &fm.get("deps").unwrap().scalars;
        assert_eq!(
            (deps[1].line, deps[1].column, deps[1].quoted),
            (5, 13, true)
        );
    }

    #[test]
    fn value_lines() {
        // ファイルの 1 行目が --- なので、YAML の 1 行目はファイルの行 1（0 始まり）
        let fm = parse("a: x\n\nb:\nc:\n  - p\nd: [q]\ne: {f: g}\n").unwrap();
        let line = |k: &str| fm.get(k).unwrap().line;
        assert_eq!(
            [line("a"), line("b"), line("c"), line("d"), line("e")],
            [1, 3, 5, 6, 7]
        );
    }

    #[test]
    fn block_scalar_at_end() {
        let fm = parse("a: |\n  x\n  y\nb: >\n  p\n  q").unwrap();
        assert_eq!(fm.get("a").unwrap().value, Value::Str("x\ny\n".into()));
        assert_eq!(fm.get("b").unwrap().value, Value::Str("p q".into()));
        let fm = parse("b: >\n  p\n  q\n").unwrap();
        assert_eq!(fm.get("b").unwrap().value, Value::Str("p q\n".into()));
    }

    #[test]
    fn duplicate_keys() {
        let fm = parse("a: 1\na: 2\n").unwrap();
        assert_eq!(fm.get("a").unwrap().value, Value::Str("1".into()));
        let entries: Vec<_> = fm.entries().collect();
        assert_eq!(entries, [("a", &Value::Str("1".into()))]);
        assert_eq!(fm.keys().collect::<Vec<_>>(), ["a", "a"]);
    }

    #[test]
    fn empty_and_not_mapping() {
        assert_eq!(parse("").unwrap(), Frontmatter::default());
        assert_eq!(parse("# comment").unwrap(), Frontmatter::default());
        assert_eq!(parse("null").unwrap(), Frontmatter::default());
        assert_eq!(parse("hello"), Err(FmError::NotMapping));
        assert_eq!(parse("- a"), Err(FmError::NotMapping));
        assert!(matches!(parse("status: [active"), Err(FmError::Yaml(_))));
    }
}
