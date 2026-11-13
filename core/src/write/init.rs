//! `desk init`: ルートの印と最小のファイルを置き、git のリポジトリにして最初のコミットを作る。

use std::fs;
use std::path::{Path, PathBuf};

use super::{Error, git_run, is_git_repo};
use crate::path::project_dir;
use crate::rules::{MARKER_FILE, MISC_PROJECT};
use crate::tree::find_marker;

/// `desk init` でしたこと。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Init {
    pub root: PathBuf,
    /// もうツリーのルートだった（何も作っていない）
    pub already: bool,
    /// 作ったもの（`.` はルートのディレクトリ、`.git/` は git のリポジトリ）
    pub created: Vec<String>,
    /// 書いたファイル（ルートからのパス）
    pub written: Vec<String>,
    /// 最初のコミットを作った
    pub committed: bool,
}

const MARKER_CONTENT: &str = "# desk のルートの印。desk は作業ディレクトリから上にたどってこれを探し、見つけたディレクトリをルートにする。\n";

const MISC_README: &str =
    "# misc\n\nどのプロジェクトにも属さないタスクを置く、なんでもありのプロジェクト。\n";

fn root_readme(name: &str) -> String {
    format!(
        "# {name}\n\n\
         あらゆるプロジェクトとタスクの作業ドキュメントを置くツリー（desk）。読み手は本人と AI エージェント。\n\n\
         ## どこに何があるか\n\n\
         ```\n\
         {name}/\n\
         ├── README.md      # これ\n\
         ├── principles/    # 全プロジェクトに通じる原則\n\
         ├── knowledge/     # 全プロジェクトで AI が使う知識\n\
         ├── archive/       # 終わったプロジェクト\n\
         └── projects/\n    \
         ├── misc/      # どのプロジェクトにも属さないタスク\n    \
         └── <project>/ # プロジェクト（tasks/・artifacts/・wiki/・inbox/・library/ など）\n\
         ```\n\n\
         ディレクトリは要るときに作る。動いているタスクや人宛ての一覧は手で書かず、`desk ls` で出す（例: `desk ls task --where needs_human=true --where status~todo,inprogress`）。\n\n\
         ## 人が書くときの決まり\n\n\
         - どこに書いてもよい\n\
         - 迷ったら `inbox/` に書く（プロジェクトが分からなければ `projects/misc/inbox/`）\n\
         - コミットはしなくてよい\n\n\
         ## 詳しく\n\n\
         決まりの正本はスキル `desk`。スキル・CLI・設計文書は desk のリポジトリにある。\n"
    )
}

/// root（絶対パス）にツリーを作る。あるファイルは書き換えず、ツリーの中には作らない。
pub fn init(root: &Path) -> Result<Init, Error> {
    let mut res = Init {
        root: root.to_path_buf(),
        already: false,
        created: Vec::new(),
        written: Vec::new(),
        committed: false,
    };
    if fs::metadata(root).is_ok_and(|m| !m.is_dir()) {
        return Err(Error::failed(format!(
            "{} はディレクトリではない",
            root.display()
        )));
    }
    match find_marker(root) {
        Some(r) if r == root => {
            res.already = true;
            return Ok(res);
        }
        Some(r) => {
            return Err(Error::failed(format!(
                "{} はツリー（{}）の中。ツリーの中にツリーは作らない",
                root.display(),
                r.display()
            )));
        }
        None => {}
    }

    if fs::metadata(root).is_err() {
        fs::create_dir_all(root)?;
        res.created.push(".".to_string());
    }
    let name = root
        .file_name()
        .map_or_else(|| "/".to_string(), |n| n.to_string_lossy().into_owned());
    let misc = project_dir(MISC_PROJECT, false);
    let files = [
        (MARKER_FILE.to_string(), MARKER_CONTENT.to_string()),
        ("README.md".to_string(), root_readme(&name)),
        (format!("{misc}/README.md"), MISC_README.to_string()),
        (".gitignore".to_string(), ".DS_Store\n".to_string()),
    ];
    for (f, content) in files {
        let p = root.join(&f);
        if fs::metadata(&p).is_ok() {
            continue;
        }
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(&p, content)?;
        res.written.push(f);
    }
    res.created.extend(res.written.iter().cloned());
    if !is_git_repo(root) {
        git_run(root, &["init", "-q", "-b", "main"])
            .map_err(|e| Error::failed(format!("git init に失敗: {e}")))?;
        res.created.push(".git/".to_string());
    }
    if git_run(root, &["rev-parse", "-q", "--verify", "HEAD"]).is_err() {
        let mut add = vec!["add", "--"];
        add.extend(res.written.iter().map(String::as_str));
        git_run(root, &add).map_err(|e| Error::failed(format!("git add に失敗: {e}")))?;
        git_run(root, &["commit", "-q", "-m", "docs: desk を作る"])
            .map_err(|e| Error::failed(format!("最初のコミットに失敗: {e}")))?;
        res.committed = true;
    }
    Ok(res)
}
