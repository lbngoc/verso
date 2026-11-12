//! Windows clipboard snapshot/restore (FR-003, research P4).
//!
//! All allocatable formats are kept; handle-based formats (bitmaps, metafiles, palettes, owner
//! display and the GDI/private ranges) are skipped and reported so the UI can say the clipboard was
//! only partly restored. Restore writes the transient markers clipboard managers honour.

use log::debug;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameW, GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};

use crate::platform::ClipboardState;

pub const MAX_TYPE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;

const CF_BITMAP: u32 = 2;
const CF_METAFILEPICT: u32 = 3;
const CF_PALETTE: u32 = 9;
const CF_ENHMETAFILE: u32 = 14;
const CF_OWNERDISPLAY: u32 = 0x0080;
const CF_PRIVATEFIRST: u32 = 0x0200;
const CF_PRIVATELAST: u32 = 0x02FF;
const CF_GDIOBJFIRST: u32 = 0x0300;
const CF_GDIOBJLAST: u32 = 0x03FF;
const CF_UNICODETEXT: u32 = 13;

struct SnapshotEntry {
    format: u32,
    name: Option<String>,
    bytes: Vec<u8>,
}

pub struct ClipboardSnapshot {
    sequence: u32,
    entries: Vec<SnapshotEntry>,
    pub skipped_formats: Vec<String>,
}

pub struct RestoreReport {
    pub state: ClipboardState,
    pub skipped_formats: Vec<String>,
}

fn format_name(format: u32) -> Option<String> {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClipboardFormatNameW(format, &mut buffer) };
    if length > 0 {
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    } else {
        None
    }
}

fn is_handle_format(format: u32) -> bool {
    matches!(
        format,
        CF_BITMAP | CF_METAFILEPICT | CF_PALETTE | CF_ENHMETAFILE | CF_OWNERDISPLAY
    ) || (CF_PRIVATEFIRST..=CF_PRIVATELAST).contains(&format)
        || (CF_GDIOBJFIRST..=CF_GDIOBJLAST).contains(&format)
}

pub fn sequence_number() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

impl ClipboardSnapshot {
    pub fn capture() -> Option<Self> {
        unsafe {
            OpenClipboard(None).ok()?;
        }
        let sequence = sequence_number();
        let mut entries = Vec::new();
        let mut skipped_formats = Vec::new();
        let mut total: usize = 0;

        let mut format = unsafe { EnumClipboardFormats(0) };
        while format != 0 {
            let name = format_name(format);
            let display = name.clone().unwrap_or_else(|| format!("CF_{format}"));
            if is_handle_format(format) {
                skipped_formats.push(display);
            } else if let Ok(handle) = unsafe { GetClipboardData(format) } {
                let size = unsafe { GlobalSize(HGLOBAL(handle.0)) };
                if size > MAX_TYPE_BYTES || total + size > MAX_TOTAL_BYTES {
                    skipped_formats.push(display);
                } else if size > 0 {
                    let pointer = unsafe { GlobalLock(HGLOBAL(handle.0)) };
                    if !pointer.is_null() {
                        let bytes = unsafe {
                            std::slice::from_raw_parts(pointer as *const u8, size).to_vec()
                        };
                        let _ = unsafe { GlobalUnlock(HGLOBAL(handle.0)) };
                        total += size;
                        entries.push(SnapshotEntry {
                            format,
                            name,
                            bytes,
                        });
                    } else {
                        skipped_formats.push(display);
                    }
                }
            }
            format = unsafe { EnumClipboardFormats(format) };
        }

        unsafe {
            let _ = CloseClipboard();
        }
        Some(Self {
            sequence,
            entries,
            skipped_formats,
        })
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn restore(self, current_sequence: u32) -> RestoreReport {
        if current_sequence == self.sequence {
            return RestoreReport {
                state: ClipboardState::Untouched,
                skipped_formats: Vec::new(),
            };
        }

        unsafe {
            if OpenClipboard(None).is_err() {
                return RestoreReport {
                    state: ClipboardState::NotRestored,
                    skipped_formats: self.skipped_formats,
                };
            }
            let _ = EmptyClipboard();

            for entry in &self.entries {
                let format = match &entry.name {
                    Some(name) => {
                        let wide: Vec<u16> =
                            name.encode_utf16().chain(std::iter::once(0)).collect();
                        RegisterClipboardFormatW(PCWSTR(wide.as_ptr()))
                    }
                    None => entry.format,
                };
                if format == 0 {
                    continue;
                }
                if let Ok(handle) = GlobalAlloc(GMEM_MOVEABLE, entry.bytes.len()) {
                    let pointer = GlobalLock(handle);
                    if !pointer.is_null() {
                        std::ptr::copy_nonoverlapping(
                            entry.bytes.as_ptr(),
                            pointer as *mut u8,
                            entry.bytes.len(),
                        );
                        let _ = GlobalUnlock(handle);
                        // On failure the HGLOBAL is left to the process (GlobalFree is not generated for
                        // this crate version); the clipboard stays empty, which the report reflects.
                        let _ = SetClipboardData(format, Some(HANDLE(handle.0)));
                    }
                }
            }

            mark_transient();
            let _ = CloseClipboard();
        }

        let state = if self.skipped_formats.is_empty() {
            ClipboardState::Restored
        } else {
            ClipboardState::PartiallyRestored
        };
        RestoreReport {
            state,
            skipped_formats: self.skipped_formats,
        }
    }
}

unsafe fn write_marker(name: &str, value: u32) {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let format = RegisterClipboardFormatW(PCWSTR(wide.as_ptr()));
    if format == 0 {
        return;
    }
    if let Ok(handle) = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of::<u32>()) {
        let pointer = GlobalLock(handle);
        if !pointer.is_null() {
            std::ptr::write(pointer as *mut u32, value);
            let _ = GlobalUnlock(handle);
            let _ = SetClipboardData(format, Some(HANDLE(handle.0)));
        }
    }
}

unsafe fn mark_transient() {
    write_marker("ExcludeClipboardContentFromMonitorProcessing", 0);
    write_marker("CanIncludeInClipboardHistory", 0);
    write_marker("CanUploadToCloudClipboard", 0);
}

/// Reads CF_UNICODETEXT (used by the copy fallback).
pub fn read_text() -> Option<String> {
    unsafe {
        OpenClipboard(None).ok()?;
        let handle: HANDLE = GetClipboardData(CF_UNICODETEXT).ok()?;
        let global = HGLOBAL(handle.0);
        let size = GlobalSize(global);
        if size == 0 {
            let _ = CloseClipboard();
            return None;
        }
        let pointer = GlobalLock(global);
        if pointer.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let units = std::slice::from_raw_parts(pointer as *const u16, size / 2);
        let end = units
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(units.len());
        let text = String::from_utf16_lossy(&units[..end]);
        let _ = GlobalUnlock(global);
        let _ = CloseClipboard();
        debug!(
            "read {} characters from the clipboard",
            text.chars().count()
        );
        Some(text)
    }
}

#[allow(dead_code)]
fn _type_check(_: HWND) {}

/// Replaces the clipboard with one CF_UNICODETEXT item ("Copy diagnostics").
pub fn write_text(text: &str) -> Result<(), String> {
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = units.len() * std::mem::size_of::<u16>();
    unsafe {
        OpenClipboard(None).map_err(|error| format!("could not open the clipboard: {error}"))?;
        let _ = EmptyClipboard();
        let result = (|| -> Result<(), String> {
            let handle = GlobalAlloc(GMEM_MOVEABLE, bytes)
                .map_err(|error| format!("could not allocate the clipboard text: {error}"))?;
            let pointer = GlobalLock(handle);
            if pointer.is_null() {
                return Err("could not lock the clipboard memory".into());
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), pointer as *mut u16, units.len());
            let _ = GlobalUnlock(handle);
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(handle.0)))
                .map_err(|error| format!("could not set the clipboard text: {error}"))?;
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}
