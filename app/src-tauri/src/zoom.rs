//! ウィンドウのズーム（`zoom:`）の動きの間も、webview が描けるようにする。
//!
//! AppKit の `zoom:`（メニューの「拡大/縮小」・option を押して緑のボタン・タイトルバーの
//! ダブルクリック・Tauri の `maximize`）は、`-[NSWindow setFrame:display:animate:]` から
//! `NSMoveHelper` の動きを、メインのランループを private なモード `_NSMoveTimerRunLoopMode` で
//! 回して行う（終わるまで戻らない）。このモードには動きの表示のタイマーしか無く、WebKit が
//! メインスレッドで受けるもの（大きさの更新・描画の結果・表示の更新の合図。どれも common modes に
//! 置かれる）が動きの間に 1 つも動かず、webview は止まって終わりに跳ぶ。
//! フィル・中央などは Dock にタイリングを頼むだけですぐ戻るので、この問題が無い。
//!
//! このモードを common modes に足すと、動きの間も common modes のものが動き、webview が
//! 窓の大きさに毎コマ追いつく（実機で測った。デザイン言語の 8 節）。マウスで端を引く大きさの変更が
//! `NSEventTrackingRunLoopMode`（もとから common modes）で回るのと同じ形になる。
//! モードの名前は公開されていない。名前が変わった OS では、足したモードが使われないだけで害は無い。

/// アプリの起動時に 1 回呼ぶ。
pub fn keep_webview_drawing_during_zoom() {
    #[cfg(target_os = "macos")]
    macos::add_move_timer_mode_to_common_modes();
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // CoreFoundation を呼ぶため。unsafe はこのモジュールの中だけに置く
mod macos {
    use std::ffi::{c_char, c_void};

    const MOVE_TIMER_MODE: &std::ffi::CStr = c"_NSMoveTimerRunLoopMode";
    const UTF8: u32 = 0x0800_0100; // kCFStringEncodingUTF8

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRunLoopGetMain() -> *mut c_void;
        fn CFRunLoopAddCommonMode(run_loop: *mut c_void, mode: *const c_void);
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            string: *const c_char,
            encoding: u32,
        ) -> *const c_void;
        fn CFRelease(object: *const c_void);
    }

    pub fn add_move_timer_mode_to_common_modes() {
        // SAFETY: CFRunLoopAddCommonMode はどのスレッドからも呼べ、名前を持ち続ける（retain する）ので、
        // 作った CFString はここで放してよい。作れなければ何もしない
        unsafe {
            let mode = CFStringCreateWithCString(std::ptr::null(), MOVE_TIMER_MODE.as_ptr(), UTF8);
            if mode.is_null() {
                return;
            }
            CFRunLoopAddCommonMode(CFRunLoopGetMain(), mode);
            CFRelease(mode);
        }
    }
}
