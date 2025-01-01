use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;

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
/// Displays a confirmation dialog box and returns true if "Yes" is clicked.
pub fn show_confirmation_box(message: &str, title: &str) -> bool {
    unsafe {
        let result = MessageBoxW(
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
            MB_YESNO | MB_ICONQUESTION,
        );

        result == windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_RESULT(6) // IDYES is defined as 6
    }
}
