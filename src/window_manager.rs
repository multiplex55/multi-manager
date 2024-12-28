use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, MessageBoxW, SetWindowPos, HWND_TOP, MB_ICONINFORMATION, MB_ICONWARNING,
    MB_OK, SWP_NOACTIVATE, SWP_NOZORDER,
};

pub fn get_active_window() -> Option<HWND> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        Some(hwnd)
    }
}

pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn listen_for_keys_with_dialog() -> Option<&'static str> {
    unsafe {
        // Display dialog prompting user input
        let message = "Press Enter to confirm or Escape to cancel.";
        MessageBoxW(
            None,
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            PCWSTR(
                "Action Required"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );

        loop {
            // Check for "Enter" key
            if GetAsyncKeyState(VK_RETURN.0 as i32) < 0 {
                return Some("Enter");
            }
            // Check for "Escape" key
            if GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 {
                MessageBoxW(
                    None,
                    PCWSTR(
                        "Action canceled by user."
                            .encode_utf16()
                            .chain(Some(0))
                            .collect::<Vec<_>>()
                            .as_ptr(),
                    ),
                    PCWSTR(
                        "Canceled"
                            .encode_utf16()
                            .chain(Some(0))
                            .collect::<Vec<_>>()
                            .as_ptr(),
                    ),
                    MB_OK | MB_ICONWARNING,
                );
                return Some("Esc");
            }
        }
    }
}
