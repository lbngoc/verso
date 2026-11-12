//! Windows selection capture: UI Automation first, simulated Ctrl+C second (research P4).

use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::debug;
use windows::core::PWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

use super::clipboard;
use crate::platform::{CaptureFailure, CaptureMethod, CaptureOutcome, ClipboardState};

const TERMINAL_PROCESSES: [&str; 10] = [
    "windowsterminal.exe",
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "conhost.exe",
    "wezterm-gui.exe",
    "alacritty.exe",
    "kitty.exe",
    "ghostty.exe",
    "openconsole.exe",
];

const KEY_C: VIRTUAL_KEY = VIRTUAL_KEY(0x43);

/// The window the shortcut fired in, captured before the popup is shown. `0` means "unknown".
static CAPTURE_TARGET: AtomicIsize = AtomicIsize::new(0);

pub fn remember_capture_target() {
    let hwnd = unsafe { GetForegroundWindow() };
    CAPTURE_TARGET.store(hwnd.0 as isize, Ordering::SeqCst);
    debug!("remembered the capture target window");
}

fn target_window() -> HWND {
    let raw = CAPTURE_TARGET.load(Ordering::SeqCst);
    if raw != 0 {
        HWND(raw as *mut core::ffi::c_void)
    } else {
        unsafe { GetForegroundWindow() }
    }
}

fn bring_target_forward(target: HWND) {
    if target.0.is_null() {
        return;
    }
    unsafe {
        let _ = SetForegroundWindow(target);
    }
    sleep(Duration::from_millis(25));
}

#[derive(Debug)]
enum ProcessLookup {
    Named(String),
    Elevated,
    Unknown,
}

fn foreground_process() -> ProcessLookup {
    process_of(target_window())
}

fn process_of(hwnd: HWND) -> ProcessLookup {
    if hwnd.0.is_null() {
        return ProcessLookup::Unknown;
    }
    let mut process_id = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
    if process_id == 0 {
        return ProcessLookup::Unknown;
    }
    let handle = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
    {
        Ok(handle) => handle,
        // A normal (non-elevated) process cannot open an elevated one: ERROR_ACCESS_DENIED.
        Err(_) => return ProcessLookup::Elevated,
    };
    let mut buffer = [0u16; 512];
    let mut length = buffer.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };
    if ok.is_err() {
        return ProcessLookup::Unknown;
    }
    let path = String::from_utf16_lossy(&buffer[..length as usize]);
    let name = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path.as_str())
        .to_string();
    ProcessLookup::Named(name)
}

pub fn frontmost_app() -> Option<String> {
    match foreground_process() {
        ProcessLookup::Named(name) => Some(name),
        _ => None,
    }
}

fn is_terminal(name: &str) -> bool {
    TERMINAL_PROCESSES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(name))
}

enum SelectionProbe {
    Text(String),
    SecureField,
    Nothing,
    Unavailable,
}

fn uia_selection() -> SelectionProbe {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let automation: IUIAutomation =
        match unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) } {
            Ok(automation) => automation,
            Err(_) => return SelectionProbe::Unavailable,
        };
    let element = match unsafe { automation.GetFocusedElement() } {
        Ok(element) => element,
        Err(_) => return SelectionProbe::Unavailable,
    };
    if unsafe { element.CurrentIsPassword() }
        .map(|value| value.as_bool())
        .unwrap_or(false)
    {
        return SelectionProbe::SecureField;
    }
    let pattern: IUIAutomationTextPattern =
        match unsafe { element.GetCurrentPatternAs(UIA_TextPatternId) } {
            Ok(pattern) => pattern,
            Err(_) => return SelectionProbe::Nothing,
        };
    let ranges = match unsafe { pattern.GetSelection() } {
        Ok(ranges) => ranges,
        Err(_) => return SelectionProbe::Nothing,
    };
    let count = unsafe { ranges.Length() }.unwrap_or(0);
    let mut text = String::new();
    for index in 0..count {
        if let Ok(range) = unsafe { ranges.GetElement(index) } {
            if let Ok(value) = unsafe { range.GetText(1_024) } {
                text.push_str(&value.to_string());
            }
        }
    }
    if text.trim().is_empty() {
        SelectionProbe::Nothing
    } else {
        SelectionProbe::Text(text)
    }
}

fn key_is_down(key: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(key.0 as i32) as u16 & 0x8000 != 0 }
}

fn wait_for_modifier_release(timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if !key_is_down(VK_MENU)
            && !key_is_down(VK_SHIFT)
            && !key_is_down(VK_LWIN)
            && !key_is_down(VK_RWIN)
        {
            return;
        }
        sleep(Duration::from_millis(10));
    }
}

fn key_input(key: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_copy() {
    // If the user already holds Ctrl, leave it down and only send the C keystroke.
    let ctrl_held = key_is_down(VK_CONTROL);
    let mut events: Vec<INPUT> = Vec::new();
    if !ctrl_held {
        events.push(key_input(VK_CONTROL, false));
    }
    events.push(key_input(KEY_C, false));
    events.push(key_input(KEY_C, true));
    if !ctrl_held {
        events.push(key_input(VK_CONTROL, true));
    }
    if !events.is_empty() {
        unsafe {
            SendInput(&events, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

pub fn capture_selection() -> CaptureOutcome {
    let started = Instant::now();
    let target = target_window();
    // The popup is shown before capture; if it took the foreground, bring the user's window back
    // so both UIA's GetFocusedElement and the Ctrl+C fallback see the selection.
    bring_target_forward(target);
    let process = foreground_process();
    let app = match &process {
        ProcessLookup::Named(name) => Some(name.clone()),
        _ => None,
    };

    if matches!(process, ProcessLookup::Elevated) {
        return CaptureOutcome::failure(CaptureFailure::ElevatedWindow, app, elapsed_ms(started));
    }

    match uia_selection() {
        SelectionProbe::Text(text) => {
            debug!("selection captured through UI Automation");
            return CaptureOutcome::success(
                CaptureMethod::Accessibility,
                text,
                elapsed_ms(started),
            );
        }
        SelectionProbe::SecureField => {
            return CaptureOutcome::failure(CaptureFailure::SecureField, app, elapsed_ms(started));
        }
        SelectionProbe::Nothing | SelectionProbe::Unavailable => {}
    }

    copy_fallback(app, started, target)
}

fn copy_fallback(app: Option<String>, started: Instant, target: HWND) -> CaptureOutcome {
    wait_for_modifier_release(Duration::from_millis(400));
    bring_target_forward(target);

    let snapshot = clipboard::ClipboardSnapshot::capture();
    let baseline = clipboard::sequence_number();
    send_copy();

    let deadline = Instant::now() + Duration::from_millis(400);
    let mut changed = false;
    while Instant::now() < deadline {
        if clipboard::sequence_number() != baseline {
            changed = true;
            break;
        }
        sleep(Duration::from_millis(5));
    }
    sleep(Duration::from_millis(10));

    let text = if changed {
        clipboard::read_text()
    } else {
        None
    };
    let (clipboard_state, skipped_formats) = match snapshot {
        Some(snapshot) => {
            let report = snapshot.restore(clipboard::sequence_number());
            (report.state, report.skipped_formats)
        }
        None => (
            ClipboardState::NotRestored,
            vec!["clipboard snapshot".to_string()],
        ),
    };

    match text {
        Some(text) if !text.trim().is_empty() => {
            let mut outcome = CaptureOutcome {
                method: CaptureMethod::Copy,
                text: Some(text),
                clipboard: clipboard_state,
                skipped_formats,
                failure: None,
                frontmost_app: app,
                duration_ms: elapsed_ms(started),
                copy_attempted: true,
            };
            if outcome.clipboard == ClipboardState::Restored && !outcome.skipped_formats.is_empty()
            {
                outcome.clipboard = ClipboardState::PartiallyRestored;
            }
            outcome
        }
        _ => CaptureOutcome {
            method: CaptureMethod::Copy,
            text: None,
            clipboard: clipboard_state,
            skipped_formats,
            failure: Some(if app.as_deref().map(is_terminal).unwrap_or(false) {
                CaptureFailure::TerminalTui
            } else {
                CaptureFailure::NothingSelected
            }),
            frontmost_app: app,
            duration_ms: elapsed_ms(started),
            copy_attempted: true,
        },
    }
}
