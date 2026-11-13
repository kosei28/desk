/// webview から呼べるアプリのコマンド。capabilities で許したものだけが呼べる。
const COMMANDS: &[&str] = &["titlebar_double_click", "index", "body", "choose_root"];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
