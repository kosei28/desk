//! 行の中の書き換え。frontmatter も本文も、YAML を書き直さずに行の置き換えで直す。

/// 行の中の書き換え 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// ファイルの行（0 始まり）
    pub line: usize,
    /// 行の中のバイト位置
    pub start: usize,
    pub old: String,
    pub new: String,
}

/// 書き換えられないとき（置換元が書いてあるものと違う）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError {
    pub line: usize,
    pub old: String,
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} 行目の {:?} を書き換えられない",
            self.line + 1,
            self.old
        )
    }
}

impl std::error::Error for EditError {}

/// 行に書き換えを当てる。同じ行の書き換えは後ろから当てるので、位置は当てる前の行で数える。
pub fn apply_edits(lines: &[String], edits: &[Edit]) -> Result<Vec<String>, EditError> {
    let mut out = lines.to_vec();
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by_key(|e| std::cmp::Reverse((e.line, e.start)));
    for e in sorted {
        let err = || EditError {
            line: e.line,
            old: e.old.clone(),
        };
        let s = out.get_mut(e.line).ok_or_else(err)?;
        let end = e.start + e.old.len();
        if s.get(e.start..end) != Some(e.old.as_str()) {
            return Err(err());
        }
        s.replace_range(e.start..end, &e.new);
    }
    Ok(out)
}
