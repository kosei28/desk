mod titlebar;
mod traffic_lights;
mod zoom;

fn main() {
    zoom::keep_webview_drawing_during_zoom();
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                traffic_lights::hide_while_exiting_fullscreen(&window)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![titlebar::titlebar_double_click])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
