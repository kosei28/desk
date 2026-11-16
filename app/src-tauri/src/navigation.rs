//! webview を、アプリの画面（index.html）の外へ移さない。
//!
//! 画面には本物の `href`（ツリーのパスと外の URL）を持つリンクがある。押したときは画面の側で止めて開き方を決めるが、
//! 右クリックのメニュー（「リンクを開く」など）・中クリック・ドラッグのように、画面の側を通らずに移ることもある。
//! それをここでまとめて止める。新しい窓は、作る処理（`on_new_window`）を渡していないので開かない。
//! 外の URL を開くのは [`crate::external::open_url`] だけ。

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime, Url};

/// どの webview にも効く、移るのを止めるプラグイン。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("navigation")
        .on_navigation(|webview, url| {
            let dev_url = if tauri::is_dev() {
                webview.config().build.dev_url.clone()
            } else {
                None
            };
            let allowed = is_app_page(url, dev_url.as_ref());
            if !allowed {
                eprintln!("webview が {url} へ移るのを止めた");
            }
            allowed
        })
        .build()
}

/// アプリの画面そのもの（読み込み・読み直し）か。開発中は devUrl、ふだんは `tauri://localhost` の index.html だけ。
/// （`tauri://localhost` のように末尾の / が無いと、パスは空になる）
/// 同じ origin でもほかのパス（ツリーのファイルを指すリンクの行き先）へは移さない。
fn is_app_page(url: &Url, dev_url: Option<&Url>) -> bool {
    let app = url.scheme() == "tauri" && url.host_str() == Some("localhost");
    let dev = dev_url.is_some_and(|dev| url.origin() == dev.origin());
    (app || dev) && matches!(url.path(), "" | "/" | "/index.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn stays_on_the_app_page() {
        let dev = url("http://localhost:1420");
        for ok in [
            "tauri://localhost",
            "tauri://localhost/",
            "tauri://localhost/index.html",
            "http://localhost:1420/",
            "http://localhost:1420/index.html?scheme=dark#top",
        ] {
            assert!(is_app_page(&url(ok), Some(&dev)), "{ok} は移ってよい");
        }
        for ng in [
            // ツリーのファイルを指すリンク（相対パスは画面の origin の下になる）
            "tauri://localhost/projects/desk/tasks/2026-11-17-desk-app-inspector.md",
            "http://localhost:1420/projects/desk/README.md",
            // 外のページ
            "https://github.com/example/recipe-app/pull/128",
            "http://localhost:1421/",
            "file:///Users/me/desk/README.md",
            "mailto:me@example.com",
        ] {
            assert!(!is_app_page(&url(ng), Some(&dev)), "{ng} へは移さない");
        }
        // ふだん（devUrl を使わない）は開発のサーバーへも移さない
        assert!(!is_app_page(&url("http://localhost:1420/"), None));
        assert!(is_app_page(&url("tauri://localhost/"), None));
    }
}
