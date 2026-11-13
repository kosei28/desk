mod titlebar;
mod zoom;

fn main() {
    zoom::keep_webview_drawing_during_zoom();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![titlebar::titlebar_double_click])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
