//! Optional-module registry stub (constitution IV). 002 ships no optional modules.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub enabled: bool,
    /// Spec that delivers the module, for the Settings hint.
    pub planned_in: String,
}

pub fn list_modules() -> Vec<ModuleInfo> {
    [
        ("ocr", "Screen OCR", "004"),
        ("offline", "Offline translation", "005"),
        ("live", "Live translation", "006"),
        ("llm", "AI translation", "006"),
    ]
    .into_iter()
    .map(|(id, name, planned_in)| ModuleInfo {
        id: id.into(),
        name: name.into(),
        available: false,
        enabled: false,
        planned_in: planned_in.into(),
    })
    .collect()
}
