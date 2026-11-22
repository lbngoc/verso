//! Text recognition with Vision (`VNRecognizeTextRequest`) and the availability of the module on
//! macOS (spec 004 R1, R2). No helper process and no model: the system recogniser is used through
//! `objc2-vision` bindings. The image never leaves memory.

use std::sync::OnceLock;

use log::{debug, info};
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::{NSArray, NSDictionary, NSProcessInfo, NSString};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};

use crate::platform::{
    OcrAvailability, OcrError, OcrLanguage, OcrUnavailableReason, RecognisedLine, Rect, RegionImage,
};

use super::screen_capture::CapturedImage;

/// The module needs macOS 14 (ScreenCaptureKit's screenshot API, R1).
const MINIMUM_MAJOR_VERSION: isize = 14;

pub fn availability() -> OcrAvailability {
    static CACHE: OnceLock<OcrAvailability> = OnceLock::new();
    CACHE.get_or_init(compute_availability).clone()
}

fn compute_availability() -> OcrAvailability {
    let version = NSProcessInfo::processInfo().operatingSystemVersion();
    if version.majorVersion < MINIMUM_MAJOR_VERSION {
        return OcrAvailability {
            available: false,
            reason: Some(OcrUnavailableReason::OsTooOld),
            languages: Vec::new(),
        };
    }
    let languages = supported_languages();
    info!("text recognition available: {} languages", languages.len());
    OcrAvailability {
        available: true,
        reason: None,
        languages,
    }
}

fn supported_languages() -> Vec<OcrLanguage> {
    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    let listed = unsafe { request.supportedRecognitionLanguagesAndReturnError() };
    match listed {
        Ok(languages) => languages
            .iter()
            .map(|code| {
                let code = code.to_string();
                OcrLanguage {
                    name: code.clone(),
                    code,
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Reads the text in a captured region. `language` is `"auto"` or a Vision language identifier.
pub fn recognise(image: RegionImage, language: &str) -> Result<Vec<RecognisedLine>, OcrError> {
    let Some(CapturedImage {
        image,
        width,
        height,
    }) = image.into_payload::<CapturedImage>()
    else {
        return Err(OcrError("the image is not a macOS capture".into()));
    };
    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }

    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    request.setUsesLanguageCorrection(true);
    if language == "auto" || language.is_empty() {
        request.setAutomaticallyDetectsLanguage(true);
    } else {
        let code = NSString::from_str(language);
        let languages = NSArray::from_retained_slice(&[code]);
        request.setRecognitionLanguages(&languages);
    }

    let options = NSDictionary::new();
    let handler = unsafe {
        VNImageRequestHandler::initWithCGImage_options(
            VNImageRequestHandler::alloc(),
            image.as_ref(),
            &options,
        )
    };
    let as_request: Retained<VNRequest> =
        Retained::into_super(Retained::into_super(request.clone()));
    let requests = NSArray::from_retained_slice(&[as_request]);
    handler
        .performRequests_error(&requests)
        .map_err(|error| OcrError(error.localizedDescription().to_string()))?;

    let (pixel_width, pixel_height) = (width as f64, height as f64);
    let mut lines = Vec::new();
    if let Some(results) = request.results() {
        for observation in results.iter() {
            let candidates = observation.topCandidates(1);
            let Some(top) = candidates.iter().next() else {
                continue;
            };
            let text = top.string().to_string();
            if text.trim().is_empty() {
                continue;
            }
            // Vision's box is normalised with a bottom-left origin; the joiner wants top-left pixels.
            let bounds = unsafe { observation.boundingBox() };
            lines.push(RecognisedLine {
                text,
                rect: Rect {
                    x: bounds.origin.x * pixel_width,
                    y: (1.0 - bounds.origin.y - bounds.size.height) * pixel_height,
                    width: bounds.size.width * pixel_width,
                    height: bounds.size.height * pixel_height,
                },
            });
        }
    }
    debug!("vision returned {} lines", lines.len());
    Ok(lines)
}
