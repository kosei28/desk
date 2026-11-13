//! ルートの下の見張り。変更が落ち着いたら知らせる。

use std::path::{Component, Path};
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};

/// 最後の変更からこれだけ何も起きなければ、落ち着いたとみなす。
const QUIET: Duration = Duration::from_millis(300);

/// 見張り。落とすと見張りをやめる。
pub type Watcher = Debouncer<RecommendedWatcher>;

/// root の下を見張り、ツリーの読み込みに関わる変更が落ち着くたびに on_change を呼ぶ。
/// root は、見張りが知らせるパスと同じ形（`canonicalize` したもの）で渡す。
pub fn watch(
    root: &Path,
    on_change: impl Fn() + Send + 'static,
) -> notify_debouncer_mini::notify::Result<Watcher> {
    let base = root.to_path_buf();
    let mut debouncer = new_debouncer(QUIET, move |result: DebounceEventResult| {
        let changed = match result {
            Ok(events) => events.iter().any(|e| concerns_tree(&base, &e.path)),
            // 何が変わったか分からないので、変わったものとして読み直させる
            Err(_) => true,
        };
        if changed {
            on_change();
        }
    })?;
    debouncer.watcher().watch(root, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// ツリーの読み込みに関わるパスか。core は `.` で始まる名前（`.git/` など）を読まないので、その下の変更は見ない。
fn concerns_tree(root: &Path, path: &Path) -> bool {
    let Ok(rest) = path.strip_prefix(root) else {
        return true;
    };
    !rest
        .components()
        .any(|c| matches!(c, Component::Normal(name) if name.to_string_lossy().starts_with('.')))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn concerns_tree_skips_dot_names() {
        let root = Path::new("/desk");
        assert!(concerns_tree(
            root,
            Path::new("/desk/projects/misc/README.md")
        ));
        assert!(concerns_tree(root, Path::new("/desk")));
        assert!(!concerns_tree(root, Path::new("/desk/.git/index")));
        assert!(!concerns_tree(root, Path::new("/desk/projects/.DS_Store")));
        // ルートの外のパスは分からないので、関わるものとして扱う
        assert!(concerns_tree(root, Path::new("/elsewhere/a.md")));
    }

    #[test]
    fn notifies_tree_changes_but_not_git() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::write(root.join(".desk"), "").unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        let (tx, rx) = mpsc::channel();
        let _watcher = watch(&root, move || tx.send(()).unwrap()).unwrap();

        fs::write(root.join(".git/index"), "x").unwrap();
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_err(),
            ".git/ の変更で知らせた"
        );

        fs::write(root.join("README.md"), "# desk\n").unwrap();
        rx.recv_timeout(Duration::from_secs(10))
            .expect("ツリーの変更で知らせない");
    }
}
