//! ルートを見つける・覚える。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use desk_core::tree::find_marker;

/// 選んだフォルダーから上にたどって、ツリーのルートを見つける（CLI がツリーの中で実行されたときと同じ）。
/// 見張りが知らせるパスと同じ形にするため、`canonicalize` して返す。
pub fn find(dir: &Path) -> Result<PathBuf, String> {
    let root = find_marker(dir).ok_or_else(|| {
        format!(
            "{} は desk のツリーの中ではない（上にたどっても .desk が無い）",
            dir.display()
        )
    })?;
    root.canonicalize()
        .map_err(|e| format!("{}: {e}", root.display()))
}

/// 覚えておいたルート。覚えていないか、もうツリーのルートでなければ None。
pub fn remembered(file: &Path) -> Option<PathBuf> {
    let root = PathBuf::from(fs::read_to_string(file).ok()?.trim_end_matches('\n'));
    (find_marker(&root).as_ref() == Some(&root)).then_some(root)
}

/// ルートを覚える。
pub fn remember(file: &Path, root: &Path) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file, format!("{}\n", root.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap().join("desk");
        fs::create_dir_all(root.join("projects/misc")).unwrap();
        fs::write(root.join(".desk"), "").unwrap();
        (dir, root)
    }

    #[test]
    fn find_walks_up_to_the_marker() {
        let (_dir, root) = tree();
        assert_eq!(find(&root.join("projects/misc")).unwrap(), root);
        assert_eq!(find(&root).unwrap(), root);
    }

    #[test]
    fn find_fails_outside_a_tree() {
        let dir = tempfile::tempdir().unwrap();
        let err = find(dir.path()).unwrap_err();
        assert!(err.contains("desk のツリーの中ではない"), "{err}");
    }

    #[test]
    fn remembers_a_root_while_it_is_still_a_root() {
        let (dir, root) = tree();
        let file = dir.path().join("config/root");
        assert_eq!(remembered(&file), None);

        remember(&file, &root).unwrap();
        assert_eq!(remembered(&file), Some(root.clone()));

        // 印が消えたら、上のツリーがあっても覚えたルートとしては使わない
        fs::write(dir.path().canonicalize().unwrap().join(".desk"), "").unwrap();
        fs::remove_file(root.join(".desk")).unwrap();
        assert_eq!(remembered(&file), None);
    }
}
