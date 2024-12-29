use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowTextW, MessageBoxW, SetWindowPos, HWND_TOP,
    MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, SWP_NOACTIVATE, SWP_NOZORDER,
};

pub fn get_active_window() -> Option<(HWND, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            None
        } else {
            let mut buffer = [0u16; 256];
            let length = GetWindowTextW(hwnd, &mut buffer);
            let title = String::from_utf16_lossy(&buffer[..length as usize]);
            Some((hwnd, title))
        }
    }
}

pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<(), &'static str> {
    unsafe {
        match SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE) {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to move window."),
        }
    }
}

pub fn get_window_position(hwnd: HWND) -> Result<(i32, i32, i32, i32), &'static str> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let x = rect.left;
            let y = rect.top;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            Ok((x, y, w, h))
        } else {
            Err("Failed to retrieve window position.")
        }
    }
}

pub fn listen_for_keys_with_dialog() -> Option<&'static str> {
    unsafe {
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
            if GetAsyncKeyState(VK_RETURN.0 as i32) < 0 {
                return Some("Enter");
            }
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
