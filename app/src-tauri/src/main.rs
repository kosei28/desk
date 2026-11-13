mod desk;
mod root;
mod titlebar;
mod traffic_lights;
mod watch;
mod zoom;

fn main() {
    zoom::keep_webview_drawing_during_zoom();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(desk::Desk::default())
        .setup(|app| {
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                traffic_lights::hide_while_exiting_fullscreen(&window)?;
            }
            // 覚えたルートを開けなくても起動し、ツリーを開いていない状態から始める
            if let Err(e) = tauri::Manager::state::<desk::Desk>(app).restore(app.handle()) {
                eprintln!("覚えておいたルートを開けない: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            titlebar::titlebar_double_click,
            desk::index,
            desk::body,
            desk::choose_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
