//! Text recognition with `Windows.Media.Ocr` (spec 004 R4). No model is bundled: the recogniser
//! uses the languages installed in Windows. The image stays in memory.

use log::{debug, info};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

use crate::platform::{
    OcrAvailability, OcrError, OcrLanguage, OcrUnavailableReason, RecognisedLine, Rect, RegionImage,
};

use super::screen_capture::CapturedImage;

fn error(context: &str, cause: windows::core::Error) -> OcrError {
    OcrError(format!("{context}: {cause}"))
}

pub fn availability() -> OcrAvailability {
    let languages = match OcrEngine::AvailableRecognizerLanguages() {
        Ok(list) => list,
        Err(error) => {
            log::warn!("could not list the OCR languages: {error}");
            return OcrAvailability {
                available: false,
                reason: Some(OcrUnavailableReason::NoLanguage),
                languages: Vec::new(),
            };
        }
    };
    let listed: Vec<OcrLanguage> = languages
        .into_iter()
        .filter_map(|language| {
            Some(OcrLanguage {
                code: language.LanguageTag().ok()?.to_string(),
                name: language
                    .DisplayName()
                    .map(|name| name.to_string())
                    .unwrap_or_default(),
            })
        })
        .collect();
    if listed.is_empty() {
        log::warn!("no OCR language is installed");
        return OcrAvailability {
            available: false,
            reason: Some(OcrUnavailableReason::NoLanguage),
            languages: listed,
        };
    }
    info!("text recognition available: {} languages", listed.len());
    OcrAvailability {
        available: true,
        reason: None,
        languages: listed,
    }
}

fn engine_for(language: &str) -> Result<OcrEngine, OcrError> {
    let engine = if language == "auto" || language.is_empty() {
        OcrEngine::TryCreateFromUserProfileLanguages()
    } else {
        let language = Language::CreateLanguage(&HSTRING::from(language))
            .map_err(|cause| error("language", cause))?;
        OcrEngine::TryCreateFromLanguage(&language)
    };
    engine.map_err(|cause| error("no recogniser for the language", cause))
}

/// Scales a BGRA buffer down (nearest neighbour) so no side exceeds `limit`.
fn fit(image: CapturedImage, limit: u32) -> (CapturedImage, f64) {
    let longest = image.width.max(image.height);
    if longest <= limit {
        return (image, 1.0);
    }
    let factor = limit as f64 / longest as f64;
    let width = ((image.width as f64 * factor).floor() as u32).max(1);
    let height = ((image.height as f64 * factor).floor() as u32).max(1);
    let mut scaled = vec![0u8; width as usize * height as usize * 4];
    for row in 0..height {
        let source_row = ((row as f64 / factor) as u32).min(image.height - 1);
        for column in 0..width {
            let source_column = ((column as f64 / factor) as u32).min(image.width - 1);
            let from = (source_row as usize * image.width as usize + source_column as usize) * 4;
            let to = (row as usize * width as usize + column as usize) * 4;
            scaled[to..to + 4].copy_from_slice(&image.bgra[from..from + 4]);
        }
    }
    (
        CapturedImage {
            width,
            height,
            bgra: scaled,
        },
        factor,
    )
}

pub fn recognise(image: RegionImage, language: &str) -> Result<Vec<RecognisedLine>, OcrError> {
    let Some(image) = image.into_payload::<CapturedImage>() else {
        return Err(OcrError("the image is not a Windows capture".into()));
    };
    let engine = engine_for(language)?;
    let limit = OcrEngine::MaxImageDimension().unwrap_or(2600);
    let (image, factor) = fit(image, limit);

    let writer = DataWriter::new().map_err(|cause| error("buffer", cause))?;
    writer
        .WriteBytes(&image.bgra)
        .map_err(|cause| error("buffer", cause))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|cause| error("buffer", cause))?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        image.width as i32,
        image.height as i32,
    )
    .or_else(|_| {
        SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            image.width as i32,
            image.height as i32,
            BitmapAlphaMode::Premultiplied,
        )
    })
    .map_err(|cause| error("bitmap", cause))?;
    // The pixel copy lives in `bitmap` now; drop ours.
    drop(image);

    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|cause| error("recognise", cause))?
        .get()
        .map_err(|cause| error("recognise", cause))?;

    let mut lines = Vec::new();
    for line in result.Lines().map_err(|cause| error("lines", cause))? {
        let text = line.Text().map(|text| text.to_string()).unwrap_or_default();
        if text.trim().is_empty() {
            continue;
        }
        // The box of a line is the union of its words' boxes, back in the original pixels.
        let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        if let Ok(words) = line.Words() {
            for word in words {
                if let Ok(bounds) = word.BoundingRect() {
                    left = left.min(bounds.X as f64);
                    top = top.min(bounds.Y as f64);
                    right = right.max((bounds.X + bounds.Width) as f64);
                    bottom = bottom.max((bounds.Y + bounds.Height) as f64);
                }
            }
        }
        if left > right {
            continue;
        }
        lines.push(RecognisedLine {
            text,
            rect: Rect {
                x: left / factor,
                y: top / factor,
                width: (right - left) / factor,
                height: (bottom - top) / factor,
            },
        });
    }
    debug!("windows ocr returned {} lines", lines.len());
    Ok(lines)
}
