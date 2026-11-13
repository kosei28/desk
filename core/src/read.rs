//! 読む: `desk read` の段 2（本文が指す先）、outline、section。段 0 の概要の整形は CLI が持つ。

use std::collections::HashSet;

use crate::doc::{Doc, Heading};
use crate::link::LinkForm;
use crate::tree::Tree;

impl Tree {
    /// 本文がリンクで指す .md（1 段、指された順、重複なし、自分は入れない）。
    pub fn linked_docs<'t>(&'t self, d: &Doc) -> Vec<&'t Doc> {
        let mut seen = HashSet::from([d.info.path.as_str()]);
        let mut out = Vec::new();
        for l in &d.links {
            if l.key != "body" || l.form != LinkForm::Relative {
                continue;
            }
            let Some(ld) = self.get(&l.target) else {
                continue;
            };
            if ld.is_md && seen.insert(ld.info.path.as_str()) {
                out.push(ld);
            }
        }
        out
    }
}

impl Heading {
    /// 節の行数（見出しの行を含み、末尾の空行は含めない）。
    pub fn lines(&self) -> usize {
        self.end - self.line
    }
}

impl Doc {
    /// frontmatter の行数（`---` の 2 行を含む）。frontmatter が無ければ None。
    pub fn fm_lines(&self) -> Option<usize> {
        self.has_fm.then_some(self.fm_end + 1)
    }

    /// その見出しの節の行（見出しの行から、末尾の空行を除いた終わりまで）。無ければ None。
    pub fn section(&self, name: &str) -> Option<&[String]> {
        self.section_range(name)
            .map(|(start, end)| &self.lines[start..end])
    }
}
