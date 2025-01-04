use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Displays a message box with the specified message and title.
///
/// This function is used to show informational messages to the user.
///
/// # Arguments
/// - `message`: The content of the message to be displayed.
/// - `title`: The title of the message box.
///
/// # Example
/// ```
/// show_message_box("Operation successful!", "Info");
/// ```
///
/// # Platform-Specific Notes
/// - This function uses the Windows API, so it is only supported on Windows.
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

/// Displays a confirmation dialog box with "Yes" and "No" options.
///
/// This function prompts the user for confirmation and returns `true` if "Yes" is clicked.
///
/// # Arguments
/// - `message`: The content of the confirmation message.
/// - `title`: The title of the confirmation dialog box.
///
/// # Returns
/// - `true` if the user selects "Yes".
/// - `false` if the user selects "No".
///
/// # Example
/// ```
/// if show_confirmation_box("Are you sure?", "Confirm Action") {
///     println!("User confirmed!");
/// } else {
///     println!("User declined.");
/// }
/// ```
///
/// # Platform-Specific Notes
/// - This function uses the Windows API, so it is only supported on Windows.
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
