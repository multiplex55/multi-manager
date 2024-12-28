use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOZORDER,
};
use winit::platform::run_return::EventLoopExtRunReturn;

pub fn get_active_window() -> Option<HWND> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd)
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

pub fn capture_hotkey_dialog(result: Arc<Mutex<Option<String>>>) {
    use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    let mut event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("Press keys for hotkey")
        .build(&event_loop)
        .expect("Failed to create window");

    let mut hotkey_parts = Vec::new();

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => match key {
                VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                    if !hotkey_parts.contains(&"Ctrl".to_string()) {
                        hotkey_parts.push("Ctrl".to_string());
                    }
                }
                VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                    if !hotkey_parts.contains(&"Shift".to_string()) {
                        hotkey_parts.push("Shift".to_string());
                    }
                }
                VirtualKeyCode::LAlt | VirtualKeyCode::RAlt => {
                    if !hotkey_parts.contains(&"Alt".to_string()) {
                        hotkey_parts.push("Alt".to_string());
                    }
                }
                VirtualKeyCode::LWin | VirtualKeyCode::RWin => {
                    if !hotkey_parts.contains(&"Win".to_string()) {
                        hotkey_parts.push("Win".to_string());
                    }
                }
                key => {
                    let key_string = format!("{:?}", key);
                    if !hotkey_parts.contains(&key_string) {
                        hotkey_parts.push(key_string);
                    }
                    *result.lock().unwrap() = Some(hotkey_parts.join("+"));
                    *control_flow = ControlFlow::Exit;
                }
            },
            _ => {}
        }
    });
}
