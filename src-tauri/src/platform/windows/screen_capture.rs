//! Region capture with GDI (spec 004 R3): the physical-pixel rectangle is copied from the screen
//! device context into a 32-bit top-down DIB section and read back as a BGRA buffer in memory.
//! The process is per-monitor DPI aware, so the rectangle is in the virtual screen's pixels.
//! Protected content comes back black, which the flow reports as "no text found". Nothing is
//! written to disk.

use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    HBITMAP, HGDIOBJ, SRCCOPY,
};

use crate::platform::{CaptureError, CaptureRegion, RegionImage};

/// BGRA pixels, top row first, `width * height * 4` bytes.
pub struct CapturedImage {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

fn failed(message: &str) -> CaptureError {
    CaptureError::Failed(message.to_string())
}

pub fn capture(region: CaptureRegion) -> Result<RegionImage, CaptureError> {
    let physical = region.physical;
    let width = physical.width.round() as i32;
    let height = physical.height.round() as i32;
    if width <= 0 || height <= 0 {
        return Err(failed("empty region"));
    }
    let (x, y) = (physical.x.round() as i32, physical.y.round() as i32);

    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err(failed("no screen device context"));
        }
        let memory = CreateCompatibleDC(Some(screen));
        if memory.is_invalid() {
            ReleaseDC(None, screen);
            return Err(failed("no memory device context"));
        }

        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap: Result<HBITMAP, _> =
            CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0);

        let result = match bitmap {
            Ok(bitmap) if !bitmap.is_invalid() => {
                let previous = SelectObject(memory, HGDIOBJ(bitmap.0));
                let copied = BitBlt(
                    memory,
                    0,
                    0,
                    width,
                    height,
                    Some(screen),
                    x,
                    y,
                    SRCCOPY | CAPTUREBLT,
                );
                let outcome = if copied.is_ok() {
                    let mut pixels = vec![0u8; width as usize * height as usize * 4];
                    let lines = GetDIBits(
                        memory,
                        bitmap,
                        0,
                        height as u32,
                        Some(pixels.as_mut_ptr().cast()),
                        &mut info,
                        DIB_RGB_COLORS,
                    );
                    if lines == 0 {
                        Err(failed("could not read the screen pixels"))
                    } else {
                        // GDI leaves the alpha byte at 0: make every pixel opaque.
                        for pixel in pixels.chunks_exact_mut(4) {
                            pixel[3] = 255;
                        }
                        Ok(RegionImage::new(CapturedImage {
                            width: width as u32,
                            height: height as u32,
                            bgra: pixels,
                        }))
                    }
                } else {
                    Err(failed("the screen copy failed"))
                };
                SelectObject(memory, previous);
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
                outcome
            }
            _ => Err(failed("could not create the capture bitmap")),
        };

        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        result
    }
}
