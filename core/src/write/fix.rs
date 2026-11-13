//! `desk check --fix`: 見つけたもののうち、機械的に 1 通りに決まる直しを当てる。

use std::collections::{HashMap, HashSet};

use super::Error;
use super::mv::{FileEdit, Move};
use crate::check::{Finding, Remedy};
use crate::edit::Edit;
use crate::tree::Tree;

/// `--fix` の計画。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixPlan {
    /// 書き換えるファイル（見つけた順）と、その書き換え
    pub edits: Vec<(String, FileEdit)>,
    /// 動かすもの（`desk archive` と同じくリンクも付け替える）
    pub moves: Vec<Move>,
}

impl FixPlan {
    /// 見つけたもののうち直せるものから計画を作る。
    pub fn new(found: &[Finding]) -> FixPlan {
        let mut plan = FixPlan::default();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for f in found {
            let mut file = |plan: &mut FixPlan| -> usize {
                *index.entry(f.path.as_str()).or_insert_with(|| {
                    plan.edits.push((f.path.clone(), FileEdit::default()));
                    plan.edits.len() - 1
                })
            };
            match &f.remedy {
                Some(Remedy::Insert(lines)) if !lines.is_empty() => {
                    let i = file(&mut plan);
                    plan.edits[i].1.inserts.extend(lines.iter().cloned());
                }
                Some(Remedy::Relink {
                    line,
                    start,
                    old,
                    new,
                }) => {
                    let i = file(&mut plan);
                    plan.edits[i].1.edits.push(Edit {
                        line: *line,
                        start: *start,
                        old: old.clone(),
                        new: new.clone(),
                    });
                }
                Some(Remedy::Move { from, to }) => plan.moves.push(Move {
                    from: from.clone(),
                    to: to.clone(),
                }),
                Some(Remedy::Insert(_)) | None => {}
            }
        }
        plan
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.moves.is_empty()
    }

    /// 計画を当てる。書き換えたファイル（動かしたものは動かした先）を返す。
    pub fn apply(&self, tree: &Tree) -> Result<Vec<String>, Error> {
        let mut changed: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for (p, fe) in &self.edits {
            let d = tree
                .get(p)
                .ok_or_else(|| Error::failed(format!("{p} はファイルでない")))?;
            fe.write(tree, d)?;
            if seen.insert(p.clone()) {
                changed.push(p.clone());
            }
        }
        if self.moves.is_empty() {
            return Ok(changed);
        }
        // 書き換えた後のツリーで動かす
        let reloaded;
        let tree = if self.edits.is_empty() {
            tree
        } else {
            reloaded = Tree::load(tree.root()).map_err(|e| Error::failed(e.to_string()))?;
            &reloaded
        };
        let res = tree.plan_moves(self.moves.clone(), &[])?.apply(tree)?;
        let to: HashMap<&str, &str> = res
            .moved
            .iter()
            .map(|m| (m.from.as_str(), m.to.as_str()))
            .collect();
        for p in &mut changed {
            if let Some(np) = to.get(p.as_str()) {
                *p = np.to_string();
                seen.insert(p.clone());
            }
        }
        for p in res.moved.iter().map(|m| &m.to).chain(&res.rewritten) {
            if seen.insert(p.clone()) {
                changed.push(p.clone());
            }
        }
        Ok(changed)
    }
}
