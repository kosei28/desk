//! ツリー全体。読むたびに走査し、保存しない。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use relative_path::RelativePath;
use serde::{Serialize, Serializer};

use crate::doc::Doc;
use crate::frontmatter::SortedFrontmatter;
use crate::link::LinkForm;
use crate::path::{Info, classify, project_dir};
use crate::rules::{ARCHIVE_DIR, Kind, MARKER_FILE, PROJECTS_DIR};

/// ツリーを読むときのエラー。
#[derive(Debug)]
pub enum Error {
    /// 上にたどっても .desk が無い
    NotInTree { dir: PathBuf },
    /// ルートが無い
    NoRoot { root: PathBuf },
    /// ルートがディレクトリでない
    RootNotDir { root: PathBuf },
    /// ファイルやディレクトリを読めない
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotInTree { dir } => write!(
                f,
                "{} はツリーの中ではない（上にたどっても {MARKER_FILE} が無い）。desk はツリーの中で実行する（新しく作るなら desk init）",
                dir.display()
            ),
            Error::NoRoot { root } => {
                write!(f, "ルート {} が無い（desk init で作る）", root.display())
            }
            Error::RootNotDir { root } => {
                write!(f, "ルート {} がディレクトリでない", root.display())
            }
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// dir から上にたどって .desk のあるディレクトリをルートとして返す。
pub fn find_root(dir: &Path) -> Result<PathBuf, Error> {
    find_marker(dir).ok_or_else(|| Error::NotInTree {
        dir: dir.to_path_buf(),
    })
}

/// dir から上にたどって .desk のあるディレクトリを返す。
pub fn find_marker(dir: &Path) -> Option<PathBuf> {
    // 末尾の / や . を落としてからたどる（返すルートの表示を作業ディレクトリの書き方に左右させない）
    let dir: PathBuf = dir.components().collect();
    dir.ancestors()
        .find(|d| d.join(MARKER_FILE).metadata().is_ok_and(|m| !m.is_dir()))
        .map(Path::to_path_buf)
}

/// リンク 1 本（書いてあるファイル → 指された先）。key は frontmatter のキーか body。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub key: String,
}

/// ツリー全体。
#[derive(Debug)]
pub struct Tree {
    root: PathBuf,
    /// パス順
    docs: Vec<Doc>,
    by_path: HashMap<String, usize>,
    /// パス → 最終コミットの日付（要るときに git から読む）
    dates: OnceLock<HashMap<String, String>>,
}

impl Tree {
    /// ルートの下を全部読む。. で始まる名前は見ない。
    pub fn load(root: &Path) -> Result<Tree, Error> {
        let meta = fs::metadata(root).map_err(|_| Error::NoRoot {
            root: root.to_path_buf(),
        })?;
        if !meta.is_dir() {
            return Err(Error::RootNotDir {
                root: root.to_path_buf(),
            });
        }
        let mut docs = Vec::new();
        walk(root, "", &mut docs)?;
        docs.sort_by(|a, b| a.info.path.cmp(&b.info.path));
        let by_path = docs
            .iter()
            .enumerate()
            .map(|(i, d)| (d.info.path.clone(), i))
            .collect();
        Ok(Tree {
            root: root.to_path_buf(),
            docs,
            by_path,
            dates: OnceLock::new(),
        })
    }

    /// ルート（渡されたとおりのパス）。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 全ファイル（パス順）。
    pub fn docs(&self) -> &[Doc] {
        &self.docs
    }

    /// ルートからのパスで引く。
    pub fn get(&self, path: &str) -> Option<&Doc> {
        self.by_path.get(path).map(|&i| &self.docs[i])
    }

    /// ルートからのパスを絶対パスにする。
    pub fn abs(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// ルートからのパス（ファイルかディレクトリ）があるか。
    pub fn exists(&self, rel: &str) -> bool {
        self.by_path.contains_key(rel) || rel == "." || self.abs(rel).exists()
    }

    /// 絶対パスをルートからのパスにする（字面だけで比べる）。ルートの外なら None。
    pub fn rel(&self, abs: &Path) -> Option<String> {
        let rest = clean_path(abs)
            .strip_prefix(clean_path(&self.root))
            .ok()?
            .to_path_buf();
        let parts: Vec<String> = rest
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        Some(if parts.is_empty() {
            ".".to_string()
        } else {
            parts.join("/")
        })
    }

    /// ファイル名（.md は省略できる）で引く。
    pub fn find_by_name(&self, name: &str) -> Vec<&Doc> {
        self.docs
            .iter()
            .filter(|d| {
                let b = d.info.name();
                b == name || b.strip_suffix(".md") == Some(name)
            })
            .collect()
    }

    /// 引数（作業ディレクトリからのパス、ルートからのパス、名前）をルートからのパスにする。
    /// 名前が複数に当たったら、作業ディレクトリのプロジェクトで絞る。
    pub fn resolve(&self, arg: &str, cwd: &Path) -> Result<String, ResolveError> {
        let arg = expand_home(arg);
        let not_found = || ResolveError::NotFound { arg: arg.clone() };
        let p = Path::new(&arg);
        if p.is_absolute() {
            let rel = self.rel(p).ok_or_else(|| ResolveError::Outside {
                arg: arg.clone(),
                root: self.root.clone(),
            })?;
            return if self.exists(&rel) {
                Ok(rel)
            } else {
                Err(not_found())
            };
        }
        let abs = cwd.join(p);
        if let Some(rel) = self.rel(&abs)
            && self.exists(&rel)
            && abs.exists()
        {
            return Ok(rel);
        }
        let c = RelativePath::new(&arg).normalize();
        if self.exists(c.as_str()) && !c.starts_with("..") {
            return Ok(c.into_string());
        }
        if arg.contains('/') {
            return Err(not_found());
        }
        let found = self.find_by_name(&arg);
        match found.len() {
            0 => return Err(not_found()),
            1 => return Ok(found[0].info.path.clone()),
            _ => {}
        }
        if let Some(rel) = self.rel(cwd)
            && rel != "."
        {
            let here = classify(&format!("{rel}/x"));
            let inside: Vec<&&Doc> = found
                .iter()
                .filter(|d| {
                    !d.info.project.is_empty()
                        && d.info.project == here.project
                        && d.info.project_archived == here.project_archived
                })
                .collect();
            if let [d] = inside[..] {
                return Ok(d.info.path.clone());
            }
        }
        Err(ResolveError::Ambiguous {
            name: arg.clone(),
            candidates: found.iter().map(|d| d.info.path.clone()).collect(),
        })
    }

    /// 最終コミットの日付（YYYY-MM-DD）。コミットされていない・git でなければ空。
    pub fn modified(&self, rel: &str) -> &str {
        self.dates
            .get_or_init(|| git_dates(&self.root))
            .get(rel)
            .map_or("", String::as_str)
    }

    /// プロジェクト（archived なら archive/ に移ったものも）。
    pub fn projects(&self, archived: bool) -> Vec<Info> {
        let mut out = Vec::new();
        let mut add = |arch: bool| {
            let parent = if arch {
                format!("{ARCHIVE_DIR}/{PROJECTS_DIR}")
            } else {
                PROJECTS_DIR.to_string()
            };
            let Ok(entries) = fs::read_dir(self.abs(&parent)) else {
                return;
            };
            let mut names: Vec<String> = entries
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('.'))
                .collect();
            names.sort();
            for name in names {
                out.push(Info {
                    path: project_dir(&name, arch),
                    kind: Some(Kind::Project),
                    project: name,
                    archived: arch,
                    project_archived: arch,
                    ..Info::default()
                });
            }
        };
        add(false);
        if archived {
            add(true);
        }
        out
    }

    /// ツリーの中のリンクのうち、先がツリーの中のファイルにあるもの（同じものは 1 本にする）。
    pub fn edges(&self) -> Vec<Edge> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for d in &self.docs {
            for l in &d.links {
                if l.form != LinkForm::Relative || l.target == d.info.path {
                    continue;
                }
                if self.get(&l.target).is_none() {
                    continue;
                }
                let e = Edge {
                    from: d.info.path.clone(),
                    to: l.target.clone(),
                    key: l.key.clone(),
                };
                if seen.insert(e.clone()) {
                    out.push(e);
                }
            }
        }
        out
    }

    /// 一覧に出るもの（`desk ls` の対象）を種類とプロジェクトで絞る。kinds が空なら全部の種類。
    /// 形の外のもの・README・静的ファイルは入れず、プロジェクトを最後に足す（プロジェクトは README を見出しに使う）。
    /// project で絞っても、ルートの原則とナレッジは全プロジェクトに当てはまるので入れる。
    pub fn entries(&self, kinds: &[Kind], project: Option<&str>, archived: bool) -> Vec<Entry<'_>> {
        let want = |k: Kind| kinds.is_empty() || kinds.contains(&k);
        let mut out: Vec<Entry<'_>> = self
            .docs
            .iter()
            .filter(|d| {
                let Some(k) = d.info.kind else { return false };
                d.is_md
                    && d.info.problem.is_none()
                    && k != Kind::Readme
                    && want(k)
                    && (archived || !d.info.archived)
                    && project.is_none_or(|p| {
                        d.info.project == p
                            || d.info.project.is_empty()
                                && matches!(k, Kind::Principle | Kind::Knowledge)
                    })
            })
            .map(|d| Entry {
                info: d.info.clone(),
                doc: Some(d),
            })
            .collect();
        if want(Kind::Project) {
            for p in self.projects(archived) {
                if project.is_some_and(|name| p.project != name) {
                    continue;
                }
                let readme = self.get(&format!("{}/README.md", p.path));
                out.push(Entry {
                    info: p,
                    doc: readme,
                });
            }
        }
        out
    }
}

/// 引数をルートからのパスにできないとき。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// ツリーの外の絶対パス
    Outside {
        arg: String,
        root: PathBuf,
    },
    NotFound {
        arg: String,
    },
    /// 名前が複数に当たった
    Ambiguous {
        name: String,
        candidates: Vec<String>,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::Outside { arg, root } => {
                write!(f, "{arg} はツリー（{}）の外", root.display())
            }
            ResolveError::NotFound { arg } => write!(f, "{arg} が無い"),
            ResolveError::Ambiguous { name, candidates } => write!(
                f,
                "{name} は複数ある。パスで指定する:\n  {}",
                candidates.join("\n  ")
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// `~` と `~/…` をホームにする。
pub(crate) fn expand_home(p: &str) -> String {
    if (p == "~" || p.starts_with("~/"))
        && let Some(home) = std::env::home_dir()
    {
        return home
            .join(p[1..].trim_start_matches('/'))
            .to_string_lossy()
            .into_owned();
    }
    p.to_string()
}

/// 字面だけで整える（. と .. を解く）。
pub(crate) fn clean_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// パス → 最終コミットの日付。git でなければ空。
fn git_dates(root: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(o) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "log",
            "--format=%x01%cs",
            "--name-only",
            "--no-renames",
        ])
        .output()
    else {
        return out;
    };
    if !o.status.success() {
        return out;
    }
    let mut date = "";
    let text = String::from_utf8_lossy(&o.stdout);
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(d) = line.strip_prefix('\x01') {
            date = d;
            continue;
        }
        out.entry(line.to_string())
            .or_insert_with(|| date.to_string());
    }
    out
}

fn walk(root: &Path, rel_dir: &str, docs: &mut Vec<Doc>) -> Result<(), Error> {
    let dir = root.join(rel_dir);
    let io = |source| Error::Io {
        path: dir.clone(),
        source,
    };
    for e in fs::read_dir(&dir).map_err(io)? {
        let e = e.map_err(io)?;
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let rel = if rel_dir.is_empty() {
            name
        } else {
            format!("{rel_dir}/{name}")
        };
        let p = e.path();
        let t = e.file_type().map_err(io)?;
        if t.is_dir() {
            walk(root, &rel, docs)?;
            continue;
        }
        // シンボリックリンクは、先が普通のファイルのときだけ読む（ディレクトリはたどらない）
        if t.is_symlink() && !fs::metadata(&p).is_ok_and(|m| m.is_file()) {
            continue;
        }
        let info = classify(&rel);
        if rel.ends_with(".md") && !info.asset {
            let b = fs::read(&p).map_err(|source| Error::Io {
                path: p.clone(),
                source,
            })?;
            let content = match String::from_utf8(b) {
                Ok(s) => s,
                Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
            };
            docs.push(Doc::parse(info, &content));
        } else {
            docs.push(Doc::file(info));
        }
    }
    Ok(())
}

/// 一覧の 1 件。ファイルか、プロジェクト（doc はその README）。
#[derive(Debug, Clone)]
pub struct Entry<'t> {
    pub info: Info,
    pub doc: Option<&'t Doc>,
}

impl Entry<'_> {
    /// 名前（プロジェクトはプロジェクトの名前、ほかはファイル名）。
    pub fn name(&self) -> &str {
        if self.info.kind == Some(Kind::Project) {
            &self.info.project
        } else {
            self.info.name()
        }
    }

    pub fn title(&self) -> &str {
        self.doc.map_or("", Doc::title)
    }

    pub fn now(&self) -> String {
        self.doc.map(Doc::now).unwrap_or_default()
    }

    /// 最終コミットの日付（プロジェクトは README の）。
    pub fn modified<'t>(&self, tree: &'t Tree) -> &'t str {
        if self.info.kind == Some(Kind::Project) {
            tree.modified(&format!("{}/README.md", self.info.path))
        } else {
            tree.modified(&self.info.path)
        }
    }
}

/// `ls --json` の 1 件の形。キーは名前の順（Go の encoding/json が map のキーを並べ替えるのに合わせる）。
impl Serialize for Entry<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Out<'a> {
            archived: bool,
            frontmatter: SortedFrontmatter<'a>,
            kind: Option<Kind>,
            name: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            now: Option<String>,
            path: &'a str,
            project: &'a str,
            title: &'a str,
        }
        Out {
            archived: self.info.archived,
            frontmatter: SortedFrontmatter(self.doc.and_then(|d| d.frontmatter.as_ref())),
            kind: self.info.kind,
            name: self.name(),
            now: (self.info.kind == Some(Kind::Task)).then(|| self.now()),
            path: &self.info.path,
            project: &self.info.project,
            title: self.title(),
        }
        .serialize(s)
    }
}
