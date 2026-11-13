//! たどる: `desk graph` の向き・キー・深さ。

use std::collections::{HashMap, HashSet};

use crate::tree::{Edge, Tree};

/// たどる向き。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 指しているもの（→）
    Out,
    /// 指されているもの（←）
    In,
}

/// たどった 1 歩（辺 1 本）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// 1 始まりの段
    pub depth: usize,
    pub dir: Direction,
    /// 辺の種類（frontmatter のキーか body）
    pub key: String,
    /// 着いた先
    pub node: String,
    /// もう出ていた（その先はたどらない）
    pub seen: bool,
}

/// たどった結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walk {
    /// たどった順の歩み（tree の形の行に当たる）
    pub steps: Vec<Step>,
    /// 初めて着いた順のパス（始まりは入れない）
    pub order: Vec<String>,
    /// 通った辺（同じ辺が 2 度入ることがある）
    pub edges: Vec<Edge>,
}

/// via が空なら全部、ほかは key が via のどれかの辺。
pub fn edges_via(edges: Vec<Edge>, via: &[String]) -> Vec<Edge> {
    edges
        .into_iter()
        .filter(|e| via.is_empty() || via.contains(&e.key))
        .collect()
}

impl Tree {
    /// start から dirs の向きに順にたどる（枝の途中で向きは変えない）。max_depth が None なら終わりまで。
    /// 辺は via で絞り、1 つの点から出る辺はキー、from、to の順に並べる。
    pub fn walk(
        &self,
        start: &str,
        via: &[String],
        dirs: &[Direction],
        max_depth: Option<usize>,
    ) -> Walk {
        let mut outs: HashMap<&str, Vec<&Edge>> = HashMap::new();
        let mut ins: HashMap<&str, Vec<&Edge>> = HashMap::new();
        let edges = edges_via(self.edges(), via);
        for e in &edges {
            outs.entry(&e.from).or_default().push(e);
            ins.entry(&e.to).or_default().push(e);
        }
        for list in outs.values_mut().chain(ins.values_mut()) {
            list.sort_by(|a, b| (&a.key, &a.from, &a.to).cmp(&(&b.key, &b.from, &b.to)));
        }
        let mut w = Walker {
            outs,
            ins,
            max: max_depth,
            seen: HashSet::from([start.to_string()]),
            walk: Walk::default(),
        };
        for &d in dirs {
            w.walk(start, d, 1);
        }
        w.walk
    }
}

struct Walker<'e> {
    outs: HashMap<&'e str, Vec<&'e Edge>>,
    ins: HashMap<&'e str, Vec<&'e Edge>>,
    max: Option<usize>,
    seen: HashSet<String>,
    walk: Walk,
}

impl Walker<'_> {
    fn walk(&mut self, node: &str, dir: Direction, depth: usize) {
        let list = match dir {
            Direction::Out => self.outs.get(node),
            Direction::In => self.ins.get(node),
        };
        let list: Vec<Edge> = list.into_iter().flatten().map(|&e| e.clone()).collect();
        for e in list {
            let next = match dir {
                Direction::Out => e.to.clone(),
                Direction::In => e.from.clone(),
            };
            let seen = !self.seen.insert(next.clone());
            self.walk.steps.push(Step {
                depth,
                dir,
                key: e.key.clone(),
                node: next.clone(),
                seen,
            });
            self.walk.edges.push(e);
            if seen {
                continue;
            }
            self.walk.order.push(next.clone());
            if self.max.is_none_or(|m| depth < m) {
                self.walk(&next, dir, depth + 1);
            }
        }
    }
}
