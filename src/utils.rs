use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

/// Displays a message box with the specified message and title.
///
/// # Arguments
///
/// * `message` - The message string to display in the message box.
/// * `title` - The title string for the message box.
pub fn show_message_box(message: &str, title: &str) {
    unsafe {
        MessageBoxW(
            HWND(ptr::null_mut()), // Null pointer for no parent window
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            PCWSTR(
                title
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
