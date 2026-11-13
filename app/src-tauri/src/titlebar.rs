//! ウィンドウの上の帯（webview の中の「タイトルバー」）のダブルクリック。
//!
//! 帯は webview の中にあるので、AppKit のタイトルバーはダブルクリックを受け取らない。
//! そこで画面から呼ばれ、AppKit のタイトルバーと同じアクション（システム設定の
//! 「タイトルバーをダブルクリックして…」に従う。ズーム・フィル・しまう・何もしない）を窓に送る。
//!
//! Tauri の `data-tauri-drag-region` のダブルクリック（`internal_toggle_maximize`）は、
//! 設定を見ずに最大化を切り替えるので使わない。ズームの動きの間に webview が止まる件は、
//! ダブルクリックではなく `zoom:` の側で直してある（`zoom.rs`）。

/// 帯のダブルクリックで画面から呼ぶ。
#[tauri::command]
pub fn titlebar_double_click(window: tauri::WebviewWindow) -> tauri::Result<()> {
    let target = window.clone();
    window.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        if let Ok(ns_window) = target.ns_window() {
            macos::perform_double_click_action(ns_window);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = target;
    })
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // AppKit を呼ぶため。unsafe はこのモジュールの中だけに置く
mod macos {
    use std::ffi::c_void;

    use objc2::runtime::{AnyClass, AnyObject, Bool, MessageReceiver, Sel};
    use objc2::{class, msg_send, sel};

    /// システム設定に従ったアクションを、メインのランループの次の回で窓に送る。
    ///
    /// `zoom:` は動きの間ランループを回し、その間も common modes のもの（tao のイベント処理を含む）が
    /// 動く（`zoom.rs`）。`run_on_main_thread` のタスクは tao のイベント処理の中で走るので、そこで
    /// `zoom:` を始めず、`performSelector:withObject:afterDelay:` でランループに積む。
    /// メニューの「拡大/縮小」と同じく、tao の処理の外でズームが動く。
    pub fn perform_double_click_action(ns_window: *mut c_void) {
        // SAFETY: Tauri の `ns_window` が返す、いま開いている NSWindow。ここはメインスレッド
        let Some(window) = (unsafe { ns_window.cast::<AnyObject>().as_ref() }) else {
            return;
        };
        let Some(action) = double_click_action(window) else {
            return;
        };
        // SAFETY: `action` は NSWindow が受ける、引数 1 つ（sender）で戻り値の無いアクション
        unsafe {
            let _: () = msg_send![
                window,
                performSelector: action,
                withObject: None::<&AnyObject>,
                afterDelay: 0.0_f64
            ];
        }
    }

    /// 「タイトルバーをダブルクリックして…」の設定から、窓に送るアクションを選ぶ。
    ///
    /// 設定の読み方は AppKit 自身の判断（`NSWindow` のクラスメソッド
    /// `_shouldMiniaturizeOnDoubleClick`・`_shouldFillOnDoubleClick`・`_shouldZoomOnDoubleClick`）に任せる。
    /// 設定のキーの読み方（既定の値や古いキー）を写さずに済み、OS が変われば AppKit と一緒に変わる。
    /// 判断を聞けない OS では、既定のズームにする。
    fn double_click_action(window: &AnyObject) -> Option<Sel> {
        let ns_window = class!(NSWindow);
        if asks(ns_window, sel!(_shouldMiniaturizeOnDoubleClick)) == Some(true) {
            return Some(sel!(miniaturize:));
        }
        if asks(ns_window, sel!(_shouldFillOnDoubleClick)) == Some(true) {
            let fill = sel!(_zoomFill:);
            return Some(if window.class().responds_to(fill) {
                fill
            } else {
                sel!(zoom:)
            });
        }
        match asks(ns_window, sel!(_shouldZoomOnDoubleClick)) {
            Some(false) => None,
            _ => Some(sel!(zoom:)),
        }
    }

    /// 引数なしで BOOL を返すクラスメソッドを聞く。無ければ `None`。
    fn asks(class: &AnyClass, question: Sel) -> Option<bool> {
        class.class_method(question)?;
        // SAFETY: `question` は引数なしで BOOL を返すクラスメソッドで、あることを上で確かめた
        let answer: Bool = unsafe { class.send_message(question, ()) };
        Some(answer.as_bool())
    }
}
