//! Region capture with ScreenCaptureKit (spec 004 R1): `SCScreenshotManager` with a filter that
//! leaves out Verso's own windows, so the overlay and the popup never appear in the picture. The
//! image goes straight to Vision as a `CGImage`; nothing is written to disk.

use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use log::debug;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_core_foundation::{CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGImage;
use objc2_foundation::{NSArray, NSError};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCRunningApplication, SCScreenshotManager, SCShareableContent,
    SCStreamConfiguration,
};

use crate::platform::{CaptureError, CaptureRegion, RegionImage};

/// `SCStreamErrorUserDeclined`: the Screen Recording permission was refused.
const USER_DECLINED: isize = -3801;
const TIMEOUT: Duration = Duration::from_secs(5);

/// The captured pixels with their size. Only this platform layer and Vision see it.
pub struct CapturedImage {
    pub image: CFRetained<CGImage>,
    pub width: usize,
    pub height: usize,
}

// SAFETY: a `CGImage` is immutable once created and Core Graphics images may be used from any
// thread; the value is handed from the capture worker to the recognition worker once.
unsafe impl Send for CapturedImage {}

fn failed(message: impl Into<String>) -> CaptureError {
    CaptureError::Failed(message.into())
}

fn denied_or_failed(error: &NSError) -> CaptureError {
    if error.code() == USER_DECLINED {
        CaptureError::PermissionMissing
    } else {
        failed(format!("ScreenCaptureKit error {}", error.code()))
    }
}

/// Fetches the shareable content (displays and running applications); blocks the worker thread.
fn shareable_content() -> Result<Retained<SCShareableContent>, CaptureError> {
    let (sender, receiver) = mpsc::channel();
    let handler = RcBlock::new(
        move |content: *mut SCShareableContent, error: *mut NSError| {
            let result = match unsafe { Retained::retain(content) } {
                Some(content) => Ok(content),
                None => Err(unsafe { error.as_ref() }
                    .map(denied_or_failed)
                    .unwrap_or_else(|| failed("no shareable content"))),
            };
            let _ = sender.send(result);
        },
    );
    unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&handler) };
    receiver
        .recv_timeout(TIMEOUT)
        .map_err(|_| failed("ScreenCaptureKit did not answer in time"))?
}

fn display_containing(
    content: &SCShareableContent,
    x: f64,
    y: f64,
) -> Option<(Retained<SCDisplay>, CGRect)> {
    let displays = unsafe { content.displays() };
    let mut first = None;
    for display in displays.iter() {
        let frame = unsafe { display.frame() };
        if first.is_none() {
            first = Some((display.clone(), frame));
        }
        let inside = x >= frame.origin.x
            && x < frame.origin.x + frame.size.width
            && y >= frame.origin.y
            && y < frame.origin.y + frame.size.height;
        if inside {
            return Some((display.clone(), frame));
        }
    }
    first
}

pub fn capture(region: CaptureRegion) -> Result<RegionImage, CaptureError> {
    let content = shareable_content()?;
    let logical = region.logical;
    let (display, frame) = display_containing(
        &content,
        logical.x + logical.width / 2.0,
        logical.y + logical.height / 2.0,
    )
    .ok_or_else(|| failed("no display found for the region"))?;

    // Verso's own windows stay out of the picture (R1).
    let own_process = std::process::id() as i32;
    let own: Vec<Retained<SCRunningApplication>> = unsafe { content.applications() }
        .iter()
        .filter(|application| unsafe { application.processID() } == own_process)
        .collect();
    let excluded = NSArray::from_retained_slice(&own);
    let windows = NSArray::new();
    let filter = unsafe {
        SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
            SCContentFilter::alloc(),
            &display,
            &excluded,
            &windows,
        )
    };

    // Source rectangle in the display's own points; the output is in physical pixels.
    let source = CGRect {
        origin: CGPoint {
            x: (logical.x - frame.origin.x).max(0.0),
            y: (logical.y - frame.origin.y).max(0.0),
        },
        size: CGSize {
            width: logical.width.min(frame.size.width),
            height: logical.height.min(frame.size.height),
        },
    };
    let width = (source.size.width * region.scale).round().max(1.0) as usize;
    let height = (source.size.height * region.scale).round().max(1.0) as usize;
    let configuration = unsafe { SCStreamConfiguration::new() };
    unsafe {
        configuration.setSourceRect(source);
        configuration.setWidth(width);
        configuration.setHeight(height);
        configuration.setShowsCursor(false);
    }

    let (sender, receiver) = mpsc::channel();
    let handler = RcBlock::new(move |image: *mut CGImage, error: *mut NSError| {
        let result = match NonNull::new(image) {
            Some(pointer) => Ok(unsafe { CFRetained::retain(pointer) }),
            None => Err(unsafe { error.as_ref() }
                .map(denied_or_failed)
                .unwrap_or_else(|| failed("no image returned"))),
        };
        let _ = sender.send(result);
    });
    unsafe {
        SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
            &filter,
            &configuration,
            Some(&handler),
        );
    }
    let image = receiver
        .recv_timeout(TIMEOUT)
        .map_err(|_| failed("the screenshot did not arrive in time"))??;
    let (pixel_width, pixel_height) = (CGImage::width(Some(&image)), CGImage::height(Some(&image)));
    debug!("captured {pixel_width}x{pixel_height} px");
    Ok(RegionImage::new(CapturedImage {
        image,
        width: pixel_width,
        height: pixel_height,
    }))
}
