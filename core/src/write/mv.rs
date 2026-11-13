//! `desk archive`・`desk mv`: ファイルを動かし、動かした後もリンクが同じ先を指すように書き換える。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::{Error, git_run, git_tracked, remove_empty_tree, remove_if_empty};
use crate::doc::Doc;
use crate::edit::{Edit, apply_edits};
use crate::frontmatter::Value;
use crate::link::LinkForm;
use crate::path::{archive_path, base, classify, dir, join, link_text, project_dir, rel_path};
use crate::rules::{ARCHIVE_DIR, Kind, PROJECTS_DIR};
use crate::tree::{Tree, expand_home};

/// 動かすファイル 1 つ（ルートからのパス）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub from: String,
    pub to: String,
}

/// ファイル 1 つの書き換え。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileEdit {
    /// 行の中の書き換え（行の順）
    pub edits: Vec<Edit>,
    /// frontmatter の最初に足す行（frontmatter が無ければ作る）
    pub inserts: Vec<String>,
}

impl FileEdit {
    fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.inserts.is_empty()
    }

    /// ファイルに当てて保存する。
    pub(crate) fn write(&self, tree: &Tree, d: &Doc) -> Result<(), Error> {
        let mut lines = apply_edits(&d.lines, &self.edits)
            .map_err(|e| Error::failed(format!("{}: {e}", d.info.path)))?;
        if !self.inserts.is_empty() {
            if d.has_fm {
                lines.splice(1..1, self.inserts.iter().cloned());
            } else {
                let mut head = vec!["---".to_string()];
                head.extend(self.inserts.iter().cloned());
                head.extend(["---".to_string(), String::new()]);
                lines.splice(0..0, head);
            }
        }
        fs::write(tree.abs(&d.info.path), lines.join("\n"))?;
        Ok(())
    }
}

/// 動かす計画。`--dry-run` はこれを出すだけ。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MovePlan {
    pub moves: Vec<Move>,
    /// 書き換えるファイル（動かす前のパスの順）と、その書き換え
    pub edits: Vec<(String, FileEdit)>,
    /// 動かした後に、中が空なら消すディレクトリ（true なら空になった親も消す）
    cleanup: Vec<(String, bool)>,
}

/// 動かした結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Moved {
    pub moved: Vec<Move>,
    /// 書き換えたファイル（動かしたものは動かした先のパス）
    pub rewritten: Vec<String>,
}

impl Moved {
    /// コミットするときに add するパス（動かした先、書き換えたもの。重なりは除く）。
    pub fn add_paths(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.moved
            .iter()
            .map(|m| &m.to)
            .chain(&self.rewritten)
            .filter(|p| seen.insert(p.as_str()))
            .cloned()
            .collect()
    }
}

/// `desk archive <パス>...` の計画。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    /// もう archive/ にあったもの（引数の順。`plan` が止まっても、そこまでに見たもの）
    pub already: Vec<String>,
    /// 動かすもの・書き換えるものが無ければ None
    pub plan: Result<Option<MovePlan>, Error>,
}

fn finished(st: &str) -> bool {
    st == "done" || st == "dropped"
}

fn is_archivable(d: &Doc) -> bool {
    matches!(d.info.kind, Some(Kind::Task | Kind::Artifact))
        || (d.info.kind == Some(Kind::Asset)
            && format!("/{}", d.info.path).contains("/artifacts/assets/"))
}

impl Tree {
    /// moves の後もリンクが同じ先を指すように書き換え、status のタスクに status を書く計画を作る。
    pub fn plan_moves(
        &self,
        moves: Vec<Move>,
        status: &[(String, String)],
    ) -> Result<MovePlan, Error> {
        let src: HashSet<&str> = moves.iter().map(|m| m.from.as_str()).collect();
        let mut dest = HashSet::new();
        for m in &moves {
            if self.get(&m.from).is_none() {
                return Err(Error::failed(format!("{} はファイルでない", m.from)));
            }
            if dest.contains(&m.to) || (self.exists(&m.to) && !src.contains(m.to.as_str())) {
                return Err(Error::failed(format!("{} は既にある", m.to)));
            }
            dest.insert(m.to.clone());
            if let Some(p) = classify(&m.to).problem {
                return Err(Error::failed(format!("{} は決まりの外の場所: {p}", m.to)));
            }
        }

        let mut edits = self.link_edits(&moves);
        for (p, s) in status {
            let fe = edits.entry(p.clone()).or_default();
            if let Some(d) = self.get(p) {
                status_edit(d, s, fe);
            }
        }
        let mut files: Vec<(String, FileEdit)> =
            edits.into_iter().filter(|(_, fe)| !fe.is_empty()).collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, fe) in &mut files {
            fe.edits.sort_by_key(|e| e.line);
        }
        Ok(MovePlan {
            moves,
            edits: files,
            cleanup: Vec::new(),
        })
    }

    /// 動かした後もリンクが同じ先を指すための書き換え（動かす前のパス → 書き換え）。
    fn link_edits(&self, moves: &[Move]) -> HashMap<String, FileEdit> {
        let to: HashMap<&str, &str> = moves
            .iter()
            .map(|m| (m.from.as_str(), m.to.as_str()))
            .collect();
        let mut out: HashMap<String, FileEdit> = HashMap::new();
        for d in self.docs().iter().filter(|d| d.is_md) {
            let moved = to.get(d.info.path.as_str()).copied();
            let new_path = moved.unwrap_or(&d.info.path);
            for l in &d.links {
                let (LinkForm::Relative, Some(start)) = (l.form, l.start) else {
                    continue;
                };
                let p = l.raw.split_once('#').map_or(l.raw.as_str(), |(p, _)| p);
                if p.is_empty() {
                    continue;
                }
                let target_moved = to.get(l.target.as_str()).copied();
                if moved.is_none() && target_moved.is_none() {
                    continue;
                }
                let new_target = target_moved.unwrap_or(&l.target);
                let new_raw = link_text(&rel_path(&dir(new_path), new_target), &l.anchor);
                if new_raw == l.raw {
                    continue;
                }
                out.entry(d.info.path.clone())
                    .or_default()
                    .edits
                    .push(Edit {
                        line: l.line,
                        start,
                        old: l.raw.clone(),
                        new: new_raw,
                    });
            }
        }
        out
    }

    /// `desk archive <パス>...`。タスクには status を書く（省略は done。done・dropped ならそのまま）。
    pub fn plan_archive(&self, args: &[&str], status: Option<&str>, cwd: &Path) -> Archive {
        let mut already = Vec::new();
        let plan = self.plan_archive_inner(args, status, cwd, &mut already);
        Archive { already, plan }
    }

    fn plan_archive_inner(
        &self,
        args: &[&str],
        status: Option<&str>,
        cwd: &Path,
        already: &mut Vec<String>,
    ) -> Result<Option<MovePlan>, Error> {
        let status = status.filter(|s| !s.is_empty());
        let mut moves = Vec::new();
        let mut st: Vec<(String, String)> = Vec::new();
        let mut seen = HashSet::new();
        for a in args {
            let p = self
                .resolve(a, cwd)
                .map_err(|e| Error::failed(e.to_string()))?;
            let Some(d) = self.get(&p) else {
                return Err(Error::failed(format!(
                    "{p} はファイルでない（プロジェクトごとなら --project）"
                )));
            };
            if !seen.insert(p.clone()) {
                continue;
            }
            let is_task = d.info.kind == Some(Kind::Task);
            if d.info.archived {
                already.push(p.clone());
                if let Some(s) = status
                    && is_task
                    && d.status() != s
                {
                    st.push((p, s.to_string()));
                }
                continue;
            }
            if !is_archivable(d) {
                return Err(Error::failed(format!(
                    "{p} は archive/ に移すものでない（移すのはタスクと artifacts/。ほかは desk mv）"
                )));
            }
            moves.push(Move {
                from: p.clone(),
                to: archive_path(&d.info),
            });
            if is_task {
                match status {
                    Some(s) => st.push((p, s.to_string())),
                    None if !finished(&d.status()) => st.push((p, "done".to_string())),
                    None => {}
                }
            }
        }
        if moves.is_empty() && st.is_empty() {
            return Ok(None);
        }
        self.plan_moves(moves, &st).map(Some)
    }

    /// `desk archive --project <名前>`。プロジェクトごとルートの archive/ へ動かす。
    pub fn plan_archive_project(&self, name: &str) -> Result<MovePlan, Error> {
        let p = project_dir(name, false);
        if name.contains('/') || !self.abs(&p).is_dir() {
            return Err(Error::usage(format!("プロジェクト {name} が無い")));
        }
        let to = project_dir(name, true);
        if self.exists(&to) {
            return Err(Error::failed(format!("{to} は既にある")));
        }
        let prefix = format!("{p}/");
        let moves: Vec<Move> = self
            .docs()
            .iter()
            .filter_map(|d| {
                let rest = d.info.path.strip_prefix(&prefix)?;
                Some(Move {
                    from: d.info.path.clone(),
                    to: format!("{to}/{rest}"),
                })
            })
            .collect();
        if moves.is_empty() {
            return Err(Error::failed(format!("{p} に動かすファイルが無い")));
        }
        let mut plan = self.plan_moves(moves, &[])?;
        plan.cleanup.push((p, false));
        Ok(plan)
    }

    /// `desk mv <元>... <先>`。先がディレクトリならその中へ、ファイルなら名前の変更。
    pub fn plan_mv(&self, srcs: &[&str], dest_arg: &str, cwd: &Path) -> Result<MovePlan, Error> {
        let dest = self.resolve_dest(dest_arg, cwd)?;
        let mut paths = Vec::new();
        for a in srcs {
            paths.push(
                self.resolve(a, cwd)
                    .map_err(|e| Error::failed(e.to_string()))?,
            );
        }
        let dest_is_dir = dest_arg.ends_with('/')
            || dest == "."
            || self.abs(&dest).is_dir()
            || !base(&dest).contains('.');
        let mut moves = Vec::new();
        if !dest_is_dir {
            match &paths[..] {
                [s] if self.get(s).is_some() => moves.push(Move {
                    from: s.clone(),
                    to: dest.clone(),
                }),
                _ => return Err(Error::usage("名前を変えるときは元のファイルを 1 つ")),
            }
        } else {
            for s in &paths {
                if self.get(s).is_some() {
                    moves.push(Move {
                        from: s.clone(),
                        to: join(&[&dest, base(s)]),
                    });
                    continue;
                }
                // ディレクトリは中のファイルを全部
                let prefix = format!("{s}/");
                let b = base(s);
                let before = moves.len();
                moves.extend(self.docs().iter().filter_map(|d| {
                    let rest = d.info.path.strip_prefix(&prefix)?;
                    Some(Move {
                        from: d.info.path.clone(),
                        to: join(&[&dest, b, rest]),
                    })
                }));
                if moves.len() == before {
                    return Err(Error::failed(format!("{s} に動かすファイルが無い")));
                }
            }
        }
        if let Some(m) = moves.iter().find(|m| m.from == m.to) {
            return Err(Error::failed(format!("{} は動かない（先が同じ）", m.from)));
        }
        let mut plan = self.plan_moves(moves, &[])?;
        plan.cleanup.extend(
            paths
                .into_iter()
                .filter(|s| self.get(s).is_none())
                .map(|s| (s, true)),
        );
        Ok(plan)
    }

    /// 動かす先の引数をルートからのパスにする（無くてよい）。
    pub fn resolve_dest(&self, arg: &str, cwd: &Path) -> Result<String, Error> {
        let arg = expand_home(arg);
        let p = Path::new(&arg);
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else if self.rel(cwd).is_some() {
            cwd.join(p)
        } else {
            self.root().join(p)
        };
        self.rel(&abs).ok_or_else(|| {
            Error::failed(format!("{arg} はツリー（{}）の外", self.root().display()))
        })
    }
}

/// task の status を書き換える（無ければ足す）。
fn status_edit(d: &Doc, status: &str, fe: &mut FileEdit) {
    let field = d.frontmatter.as_ref().and_then(|fm| fm.get("status"));
    let Some(field) = field else {
        fe.inserts.push(format!("status: {status}"));
        return;
    };
    if let Value::Str(v) = &field.value {
        if v == status {
            return;
        }
        if !v.is_empty()
            && let Some(sc) = field.scalars.first()
            && let Some(start) = d.locate(sc)
        {
            fe.edits.push(Edit {
                line: sc.line,
                start,
                old: v.clone(),
                new: status.to_string(),
            });
            return;
        }
    }
    // null と書いた値（`~`・`null`）: その字だけを置き換える
    if field.value == Value::Null
        && let Some((start, tok)) = d.lines.get(field.line).and_then(|l| null_token(l))
    {
        fe.edits.push(Edit {
            line: field.line,
            start,
            old: tok.to_string(),
            new: status.to_string(),
        });
        return;
    }
    // 空の status: 行ごと置き換える
    for i in 1..d.fm_end {
        if d.lines[i].trim().starts_with("status:") {
            fe.edits.push(Edit {
                line: i,
                start: 0,
                old: d.lines[i].clone(),
                new: format!("status: {status}"),
            });
            return;
        }
    }
    fe.inserts.push(format!("status: {status}"));
}

/// 値の行にある null の字（`~`・`null` など）の位置と字。`status:` の行ならその後ろを見る。
fn null_token(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    let mut off = line.len() - t.len();
    if t.starts_with("status:") {
        off += "status:".len();
    }
    let rest = &line[off..];
    let start = off + rest.len() - rest.trim_start().len();
    let tok = line[start..].split(char::is_whitespace).next()?;
    matches!(tok, "~" | "null" | "Null" | "NULL").then_some((start, tok))
}

impl MovePlan {
    /// 計画を当てる: ファイルを書き換え、動かし（git で管理していれば git mv）、空になったディレクトリを消す。
    pub fn apply(&self, tree: &Tree) -> Result<Moved, Error> {
        let new_path: HashMap<&str, &str> = self
            .moves
            .iter()
            .map(|m| (m.from.as_str(), m.to.as_str()))
            .collect();
        let mut res = Moved {
            moved: self.moves.clone(),
            rewritten: Vec::new(),
        };
        for (p, fe) in &self.edits {
            let d = tree
                .get(p)
                .ok_or_else(|| Error::failed(format!("{p} はファイルでない")))?;
            fe.write(tree, d)?;
            res.rewritten.push(
                new_path
                    .get(p.as_str())
                    .map_or(p.clone(), |n| n.to_string()),
            );
        }
        let root = tree.root();
        for m in &self.moves {
            if let Some(d) = tree.abs(&m.to).parent() {
                fs::create_dir_all(d)?;
            }
            if git_tracked(root, &m.from) {
                git_run(root, &["mv", "--", &m.from, &m.to])
                    .map_err(|e| Error::failed(format!("git mv {} {}: {e}", m.from, m.to)))?;
            } else {
                fs::rename(tree.abs(&m.from), tree.abs(&m.to))?;
            }
        }
        for m in &self.moves {
            remove_empty_parents(tree, &dir(&m.from));
        }
        for (d, parents) in &self.cleanup {
            remove_empty_tree(&tree.abs(d));
            if *parents {
                remove_empty_parents(tree, &dir(d));
            }
        }
        Ok(res)
    }
}

/// 空になったディレクトリを消す（プロジェクトのディレクトリと、ルートの予約名のディレクトリは残す）。
fn remove_empty_parents(tree: &Tree, start: &str) {
    let mut d = start.to_string();
    while d != "." && !d.is_empty() {
        let parts: Vec<&str> = d.split('/').collect();
        if parts.len() < 2
            || (parts[0] == PROJECTS_DIR && parts.len() <= 2)
            || (parts[0] == ARCHIVE_DIR && parts.len() <= 3)
        {
            return;
        }
        if !remove_if_empty(&tree.abs(&d)) {
            return;
        }
        d = dir(&d);
    }
}
