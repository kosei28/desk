//! 開いているツリーと、webview に出すコマンド。
//!
//! ルートは 1 つで、Rust の側だけが持つ。webview はパスを渡さず、コマンドで中身をもらう。
//! ツリーが変わったら [`TREE_CHANGED`] を送る。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use desk_core::{Doc, Tree};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use crate::root;
use crate::watch::{self, Watcher};

/// ツリーが変わった（変更が落ち着いた）ときに webview に送るイベント。中身は持たない。
pub const TREE_CHANGED: &str = "tree-changed";

/// 開いているツリー。
#[derive(Default)]
pub struct Desk {
    opened: Mutex<Option<Opened>>,
}

struct Opened {
    root: PathBuf,
    _watcher: Watcher,
}

impl Desk {
    /// 起動のときに呼ぶ。覚えておいたルートがあれば開く。
    pub fn restore<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        match root::remembered(&remembered_file(app)?) {
            Some(root) => self.open(app, root),
            None => Ok(()),
        }
    }

    /// root を開き、見張りを始める。前のツリーの見張りはやめる。
    fn open<R: Runtime>(&self, app: &AppHandle<R>, root: PathBuf) -> Result<(), String> {
        let app = app.clone();
        let watcher = watch::watch(&root, move || {
            let _ = app.emit(TREE_CHANGED, ());
        })
        .map_err(|e| format!("{} を見張れない: {e}", root.display()))?;
        *self.opened.lock().unwrap() = Some(Opened {
            root,
            _watcher: watcher,
        });
        Ok(())
    }

    fn root(&self) -> Option<PathBuf> {
        self.opened.lock().unwrap().as_ref().map(|o| o.root.clone())
    }
}

/// ルートを覚えておくファイル。
fn remembered_file<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("root"))
}

/// 索引: ルートと、ルートの下の全ファイル（パス順。`.` で始まる名前は入れない）。
pub struct Index(Tree);

impl Index {
    pub fn load(root: &Path) -> Result<Index, String> {
        Tree::load(root).map(Index).map_err(|e| e.to_string())
    }

    /// 索引にある .md の本文（frontmatter を外したもの）。索引に無いパスや .md でないものはエラー。
    fn body(&self, path: &str) -> Result<String, String> {
        match self.0.get(path) {
            Some(doc) if doc.is_md => Ok(doc.lines[doc.body_start..].join("\n")),
            _ => Err(format!("{path} は索引にある Markdown のファイルではない")),
        }
    }
}

impl Serialize for Index {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut out = s.serialize_struct("Index", 2)?;
        out.serialize_field("root", &self.0.root().to_string_lossy())?;
        out.serialize_field::<[Doc]>("docs", self.0.docs())?;
        out.end()
    }
}

/// 開いているツリーの索引。ツリーを開いていなければ null。
#[tauri::command(async)]
pub fn index(desk: State<'_, Desk>) -> Result<Option<Index>, String> {
    desk.root().map(|root| Index::load(&root)).transpose()
}

/// 開いているツリーの、ルートからのパスの Markdown の本文（frontmatter を外したもの）。
/// 読むのは索引にある .md だけ。
#[tauri::command(async)]
pub fn body(desk: State<'_, Desk>, path: String) -> Result<String, String> {
    let root = desk.root().ok_or("ツリーを開いていない")?;
    Index::load(&root)?.body(&path)
}

/// フォルダーを選ぶ窓を出し、選んだフォルダーのあるツリーを開いて覚える。選ばずに閉じたら何もしない。
#[tauri::command]
pub async fn choose_root<R: Runtime>(
    app: AppHandle<R>,
    desk: State<'_, Desk>,
) -> Result<(), String> {
    let dialog = app.dialog().file();
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_folder())
        .await
        .map_err(|e| e.to_string())?;
    let Some(picked) = picked else {
        return Ok(());
    };
    let dir = picked.into_path().map_err(|e| e.to_string())?;
    let root = root::find(&dir)?;
    desk.open(&app, root.clone())?;
    root::remember(&remembered_file(&app)?, &root).map_err(|e| format!("ルートを覚えられない: {e}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    #[test]
    fn body_strips_frontmatter_and_reads_only_markdown_in_the_index() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("desk");
        write(&root, ".desk", "");
        write(&root, ".git/notes.md", "# git\n");
        write(dir.path(), "outside.md", "# 外\n");
        write(&root, "README.md", "# desk\n\n架空のツリー。\n");
        write(
            &root,
            "projects/misc/tasks/2026-01-02-tidy.md",
            "---\r\nstatus: todo\r\n---\r\n\r\n# 片付ける\r\n",
        );
        write(&root, "projects/misc/artifacts/assets/note.md", "# 静的\n");
        write(&root, "projects/misc/artifacts/assets/photo.png", "");
        let index = Index::load(&root).unwrap();

        assert_eq!(
            index.body("README.md").unwrap(),
            "# desk\n\n架空のツリー。\n"
        );
        assert_eq!(
            index
                .body("projects/misc/tasks/2026-01-02-tidy.md")
                .unwrap(),
            "\n# 片付ける\n"
        );
        for path in [
            "../outside.md",
            "projects/misc/tasks/../../../README.md",
            ".git/notes.md",
            "projects/misc/artifacts/assets/note.md",
            "projects/misc/artifacts/assets/photo.png",
            "projects/misc",
            "missing.md",
            "/etc/hosts",
        ] {
            let err = index.body(path).unwrap_err();
            assert!(
                err.contains("索引にある Markdown のファイルではない"),
                "{path}: {err}"
            );
        }
    }

    #[test]
    fn index_serializes_root_and_docs_in_path_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("desk");
        let write = |rel: &str, content: &str| write(&root, rel, content);
        write(".desk", "");
        write(".git/HEAD", "ref: refs/heads/main\n");
        write("README.md", "# desk\n");
        write(
            "projects/misc/tasks/2026-01-02-tidy.md",
            "---\nstatus: todo\nneeds_human: false\n---\n\n# 片付ける\n\n## いま\n- まだ\n",
        );
        write("projects/misc/artifacts/assets/photo.png", "");

        let json = serde_json::to_value(Index::load(&root).unwrap()).unwrap();
        assert_eq!(json["root"], root.to_string_lossy().as_ref());
        let paths: Vec<&str> = json["docs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            [
                "README.md",
                "projects/misc/artifacts/assets/photo.png",
                "projects/misc/tasks/2026-01-02-tidy.md",
            ]
        );
        let task = &json["docs"][2];
        assert_eq!(task["kind"], "task");
        assert_eq!(task["title"], "片付ける");
        assert_eq!(task["status"], "todo");
        assert_eq!(task["now"], "まだ");
        assert_eq!(
            task["frontmatter"][0],
            serde_json::json!({ "key": "status", "value": "todo" })
        );
    }
}
