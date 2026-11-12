//! Text-to-speech through the `tts` crate (AVSpeechSynthesizer on macOS).

use std::sync::{Mutex, OnceLock};

use log::debug;

fn engine() -> Result<&'static Mutex<tts::Tts>, String> {
    static ENGINE: OnceLock<Mutex<tts::Tts>> = OnceLock::new();
    if let Some(engine) = ENGINE.get() {
        return Ok(engine);
    }
    let created = engine_create()?;
    let _ = ENGINE.set(created);
    ENGINE
        .get()
        .ok_or_else(|| "speech engine unavailable".to_string())
}

fn engine_create() -> Result<Mutex<tts::Tts>, String> {
    tts::Tts::default()
        .map(Mutex::new)
        .map_err(|error| format!("speech engine unavailable: {error}"))
}

fn language_matches(voice_language: &str, wanted: &str) -> bool {
    let voice = voice_language.to_ascii_lowercase().replace('_', "-");
    let wanted = wanted.to_ascii_lowercase().replace('_', "-");
    voice == wanted || voice.starts_with(&format!("{wanted}-"))
}

pub fn voice_for(lang: &str) -> Option<tts::Voice> {
    let guard = engine().ok()?;
    let tts = guard.lock().ok()?;
    let voices = tts.voices().ok()?;
    voices
        .into_iter()
        .find(|voice| language_matches(voice.language().as_ref(), lang))
}

pub fn voice_exists(lang: &str) -> bool {
    voice_for(lang).is_some()
}

pub fn speak(text: &str, lang: &str) -> Result<(), String> {
    let voice = voice_for(lang).ok_or_else(|| format!("no voice installed for {lang}"))?;
    let guard = engine()?;
    let mut tts = guard
        .lock()
        .map_err(|_| "speech engine is busy".to_string())?;
    tts.set_voice(&voice)
        .map_err(|error| format!("could not select the voice: {error}"))?;
    tts.speak(text, true)
        .map(|_| ())
        .map_err(|error| format!("could not speak: {error}"))
}

/// Logged once at startup so the UI can disable Speak without asking the engine each time.
pub fn log_voice_availability(lang: &str) -> bool {
    let available = voice_exists(lang);
    debug!("voice for {lang}: {available}");
    available
}
