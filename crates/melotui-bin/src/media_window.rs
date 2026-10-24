//! A hidden window for Windows' media controls to belong to.
//!
//! Windows ties its media overlay and media keys to a window, and a terminal
//! app has none of its own; this one is never shown, and a thread of its own
//! pumps its messages for as long as the app runs.

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HMENU, MSG, RegisterClassW,
    TranslateMessage, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};
use windows::w;

unsafe extern "system" fn procedure(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(window, message, w, l) }
}

/// Creates the window, returning its handle, or `None` if it could not be.
pub fn spawn() -> Option<isize> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("media-window".into())
        .spawn(move || unsafe {
            let Ok(module) = GetModuleHandleW(None) else {
                let _ = tx.send(None);
                return;
            };
            let instance = HINSTANCE(module.0);
            let class = w!("melotui-media-controls");
            let description = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&description);
            let window = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                w!("melotui"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                0,
                0,
                HWND(0),
                HMENU(0),
                instance,
                None,
            );
            if window.0 == 0 {
                let _ = tx.send(None);
                return;
            }
            let _ = tx.send(Some(window.0));
            let mut message = MSG::default();
            while GetMessageW(&mut message, HWND(0), 0, 0).as_bool() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        })
        .ok()?;
    rx.recv().ok().flatten()
}
