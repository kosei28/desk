//! 信号機（閉じる・しまう・ズームのボタン）を、全画面から戻る動きの間だけ隠す。
//!
//! 位置（`tauri.conf.json` の `trafficLightPosition`）は tao が置く。AppKit がボタンを既定の位置（左上 9px）に
//! 置き直しても、tao が内容のビューの `drawRect:`・全画面から戻った後（`windowDidExitFullScreen:`）・タイトルの
//! 変更の後にかけ直す。
//!
//! 全画面から戻る動きの間に画面に出ているのは、AppKit が別の窓に載せた写しで、戻った後の 1 枚は、AppKit が
//! 窓を戻してボタンを既定の位置に置いた直後に撮る。窓の大きさが変わらない（ズームした窓の）ときは、それが
//! 同じランループの 1 回の中で同期に起き、tao のかけ直しより前になる。そこで Electron と同じく、戻り始める
//! 通知（`NSWindowWillExitFullScreenNotification`）でボタンの入れ物を隠して写しに写さず、戻り終えた通知
//! （`NSWindowDidExitFullScreenNotification`）で出す。戻り終えた通知と tao のかけ直しは同じ呼び出しの中で
//! 走るので、どちらが先でも、画面に出るときには設定の位置にある。

/// メインの窓で、全画面から戻る動きの間だけ信号機を隠す。起動の `setup` で 1 回呼ぶ。
pub fn hide_while_exiting_fullscreen(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    macos::hide_while_exiting_fullscreen(window.ns_window()?);
    #[cfg(not(target_os = "macos"))]
    let _ = window;
    Ok(())
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // AppKit を呼ぶため。unsafe はこのモジュールの中だけに置く
mod macos {
    use std::ffi::c_void;
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSWindow, NSWindowButton, NSWindowDidExitFullScreenNotification,
        NSWindowWillExitFullScreenNotification,
    };
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName};

    /// 信号機のボタンが入っている入れ物（タイトルバーの入れ物）を隠す・出す。
    fn set_hidden(window: &NSWindow, hidden: bool) {
        let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
            return;
        };
        // SAFETY: メインスレッドで、窓が持つビューの親をたどるだけ。返るビューは retain されている
        let Some(container) = (unsafe { close.superview().and_then(|view| view.superview()) })
        else {
            return;
        };
        container.setHidden(hidden);
    }

    /// 窓の `name` の通知で、入れ物を `hidden` にする。受け取りはアプリが終わるまで外さない。
    fn on(window: &Retained<NSWindow>, name: &NSNotificationName, hidden: bool) {
        let target = window.clone();
        // 通知は AppKit の中から同期で届く（queue を渡さないので、送った側のスレッド、つまりメインスレッドで走る）
        let block = RcBlock::new(move |_: NonNull<NSNotification>| set_hidden(&target, hidden));
        // SAFETY: object は窓で、その窓の通知だけを受ける。返る token は放さずに残す
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                Some(window),
                None,
                &block,
            )
        };
        std::mem::forget(token);
    }

    pub fn hide_while_exiting_fullscreen(ns_window: *mut c_void) {
        // SAFETY: Tauri の `ns_window` が返す、いま開いている NSWindow。setup はメインスレッドで走る。
        // retain して、通知の受け取りが持ち続ける（窓はアプリと同じだけ生きる）
        let Some(window) = (unsafe { Retained::retain(ns_window.cast::<NSWindow>()) }) else {
            return;
        };
        // SAFETY: 通知の名前は AppKit の定数
        let (will, did) = unsafe {
            (
                NSWindowWillExitFullScreenNotification,
                NSWindowDidExitFullScreenNotification,
            )
        };
        on(&window, will, true);
        on(&window, did, false);
    }
}
