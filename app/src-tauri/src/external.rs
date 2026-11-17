//! 外の URL を、既定のアプリ（ブラウザ・メール）で開く。
//!
//! webview の中では外のページへ移らない。開いてよいのは [`SCHEMES`] の URL だけで、ここで確かめる。
//! 押されるのは本文と図の中のリンクと、右の列の URL（どれも同じ scheme だけを押せるようにしている）。

use std::process::Command;

use tauri::Url;

/// 開いてよい scheme。本文の消毒（rehype-sanitize）と図（pele）がリンクに残す既定の scheme を合わせたもの。
/// TS の `urlSchemes`（src/data/url-schemes.ts）と同じ並びにする
const SCHEMES: &[&str] = &["http", "https", "mailto", "tel", "irc", "ircs", "xmpp"];

/// 開いてよい URL なら、解いたものを返す。
fn checked(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|e| format!("{raw} は URL ではない: {e}"))?;
    if !SCHEMES.contains(&url.scheme()) {
        return Err(format!(
            "{raw} は開かない（開くのは {} だけ）",
            SCHEMES.join("・")
        ));
    }
    Ok(url)
}

/// 外の URL を既定のアプリで開く。[`SCHEMES`] の URL だけ。
#[tauri::command(async)]
pub fn open_url(url: String) -> Result<(), String> {
    let url = checked(&url)?;
    // 解いた URL は scheme から始まるので、open のオプションには読まれない
    let status = Command::new("/usr/bin/open")
        .arg(url.as_str())
        .status()
        .map_err(|e| format!("open を起動できない: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{url} を開けない（open が {status} で終わった）"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_only_the_link_schemes() {
        for ok in [
            "https://github.com/example/recipe-app/pull/128",
            "http://example.com/a",
            "mailto:someone@example.com",
            "tel:+81-3-0000-0000",
            "irc://irc.example.com/recipes",
            "ircs://irc.example.com:6697/recipes",
            "xmpp:planner@example.com",
        ] {
            assert!(checked(ok).is_ok(), "{ok}");
        }
        for ng in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "x-apple.systempreferences:com.apple.preference.security",
            "slack://open",
            "ftp://example.com/a",
            "-a Calculator",
            "../tasks/a.md",
            "",
        ] {
            assert!(checked(ng).is_err(), "{ng}");
        }
    }
}
