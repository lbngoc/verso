//! Tray icon and menu: Translate Selection, Quick Input, Settings, Quit (FR-001).
//!
//! The mark comes from `scripts/gen-tray-icon.py` (spec 008). macOS gets the black glyph as a template
//! image, which the system tints for light, dark and tinted menu bars. Windows does not tint tray
//! icons, so it gets the white glyph on a dark taskbar and the black one on a light taskbar, swapped
//! when the theme changes (T070). While a translation runs the mark switches to its busy variant (T071).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

use parking_lot::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

use crate::shortcuts;
use crate::window_manager;

const TRAY_ID: &str = "main";
const TOOLTIP_IDLE: &str = "Verso";
const TOOLTIP_BUSY: &str = "Verso — translating…";

// The macOS status item scales its image to an 18 pt height, so the 18 px grid at @2x is drawn 1:1
// on Retina and halves cleanly on 1x screens.
#[cfg(target_os = "macos")]
mod art {
    pub const IDLE_DARK_BAR: &[u8] = include_bytes!("../icons/tray/tray-18@2x.png");
    pub const BUSY_DARK_BAR: &[u8] = include_bytes!("../icons/tray/tray-18-busy@2x.png");
    // Template images: the same black glyph serves both themes.
    pub const IDLE_LIGHT_BAR: &[u8] = IDLE_DARK_BAR;
    pub const BUSY_LIGHT_BAR: &[u8] = BUSY_DARK_BAR;
}

// The notification area shows 16 px icons at 100 % scaling; the 16 px grid at @2x halves exactly at
// 100 % and is 1:1 at 200 %.
#[cfg(not(target_os = "macos"))]
mod art {
    pub const IDLE_DARK_BAR: &[u8] = include_bytes!("../icons/tray/tray-16-light@2x.png");
    pub const BUSY_DARK_BAR: &[u8] = include_bytes!("../icons/tray/tray-16-busy-light@2x.png");
    pub const IDLE_LIGHT_BAR: &[u8] = include_bytes!("../icons/tray/tray-16@2x.png");
    pub const BUSY_LIGHT_BAR: &[u8] = include_bytes!("../icons/tray/tray-16-busy@2x.png");
}

/// Number of translations in flight, so overlapping requests don't flip the mark back to idle early.
static BUSY: AtomicU8 = AtomicU8::new(0);
static DARK_BAR: AtomicBool = AtomicBool::new(true);
/// The version the updater offered, shown as its own item (FR-015, revision 3).
static UPDATE_VERSION: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn update_version() -> &'static Mutex<Option<String>> {
    UPDATE_VERSION.get_or_init(|| Mutex::new(None))
}

/// Shows or clears the "Update available" item as the updater learns about a release.
pub fn set_update_available(app: &AppHandle, version: Option<&str>) {
    let next = version.map(str::to_string);
    if *update_version().lock() == next {
        return;
    }
    *update_version().lock() = next;
    // Menu calls belong on the main thread; the updater runs on a worker.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id(TRAY_ID) else {
            return;
        };
        match build_menu(&handle) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(error) => log::warn!("tray menu could not be rebuilt: {error}"),
        }
    });
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let translate = MenuItem::with_id(app, "translate", "Translate Selection", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "Quick Input…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Verso", true, None::<&str>)?;
    let report = MenuItem::with_id(app, "report", "Report a problem…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Verso", true, None::<&str>)?;

    let update = update_version()
        .lock()
        .clone()
        .map(|version| {
            MenuItem::with_id(
                app,
                "update",
                format!("Update available — v{version}"),
                true,
                None::<&str>,
            )
        })
        .transpose()?;

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        vec![&translate, &quick, &settings, &about];
    if let Some(update) = &update {
        items.push(update);
    }
    items.push(&report);
    items.push(&separator);
    items.push(&quit);
    Menu::with_items(app, &items)
}

/// sRGB relative luminance (WCAG 2.x), used only to tell a dark theme background from a light one.
#[cfg(any(windows, test))]
fn relative_luminance((red, green, blue): (u8, u8, u8)) -> f64 {
    fn channel(value: u8) -> f64 {
        let value = f64::from(value) / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
}

/// Whether the tray sits on a dark bar, so it needs the white glyph.
///
/// Normal themes follow `SystemUsesLightTheme` (0 or missing means dark). High-contrast themes repaint the
/// taskbar while that value keeps the normal theme's setting, so the glyph follows the luminance of the
/// theme background instead; without a readable colour the white glyph is the safe choice (research R9).
#[cfg(any(windows, test))]
pub(crate) fn dark_bar(
    high_contrast: bool,
    high_contrast_background: Option<(u8, u8, u8)>,
    system_uses_light_theme: Option<u32>,
) -> bool {
    if high_contrast {
        return high_contrast_background
            .map(|rgb| relative_luminance(rgb) < 0.5)
            .unwrap_or(true);
    }
    !matches!(system_uses_light_theme, Some(value) if value != 0)
}

fn bar_is_dark() -> bool {
    #[cfg(windows)]
    {
        crate::platform::windows::appearance::taskbar_is_dark()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn art_for(busy: bool, dark_bar: bool) -> &'static [u8] {
    match (busy, dark_bar) {
        (false, true) => art::IDLE_DARK_BAR,
        (true, true) => art::BUSY_DARK_BAR,
        (false, false) => art::IDLE_LIGHT_BAR,
        (true, false) => art::BUSY_LIGHT_BAR,
    }
}

fn apply(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let busy = BUSY.load(Ordering::SeqCst) > 0;
    let dark_bar = DARK_BAR.load(Ordering::SeqCst);
    match Image::from_bytes(art_for(busy, dark_bar)) {
        Ok(icon) => {
            let _ = tray.set_icon(Some(icon));
            #[cfg(target_os = "macos")]
            let _ = tray.set_icon_as_template(true);
        }
        Err(error) => log::warn!("tray icon could not be decoded: {error}"),
    }
    let _ = tray.set_tooltip(Some(if busy { TOOLTIP_BUSY } else { TOOLTIP_IDLE }));
}

/// Marks a translation as started (`true`) or finished, whether it succeeded or failed (`false`).
pub fn set_busy(app: &AppHandle, busy: bool) {
    // Only the first start and the last finish change what the tray shows.
    let changed = if busy {
        BUSY.fetch_add(1, Ordering::SeqCst) == 0
    } else {
        // Every `false` follows its own `true` (see shortcuts::trigger_translate).
        BUSY.fetch_sub(1, Ordering::SeqCst) == 1
    };
    if changed {
        apply(app);
    }
}

/// Re-reads the taskbar theme and swaps the mark if it changed. Called by the appearance watcher.
pub fn refresh_theme(app: &AppHandle) {
    let dark_bar = bar_is_dark();
    if DARK_BAR.swap(dark_bar, Ordering::SeqCst) != dark_bar {
        log::debug!("tray mark follows the taskbar theme (dark: {dark_bar})");
        apply(app);
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let dark_bar = bar_is_dark();
    DARK_BAR.store(dark_bar, Ordering::SeqCst);
    let icon = Image::from_bytes(art_for(false, dark_bar))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(TOOLTIP_IDLE)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "translate" => shortcuts::trigger_translate(app),
            "quick" => {
                let _ = window_manager::show_quick(app);
            }
            "settings" => window_manager::show_settings(app),
            "about" => window_manager::show_settings_section(app, "about"),
            "update" => window_manager::show_settings_section(app, "about"),
            "report" => {
                if let Err(error) = crate::commands::report_problem(app.clone()) {
                    log::warn!("problem report could not be opened: {error}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                shortcuts::trigger_translate(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_decodes_and_is_square() {
        for busy in [false, true] {
            for dark_bar in [false, true] {
                let image = Image::from_bytes(art_for(busy, dark_bar)).expect("tray PNG decodes");
                assert_eq!(image.width(), image.height());
                assert!(image.width() >= 32, "tray art is the @2x grid");
            }
        }
    }

    #[test]
    fn busy_art_differs_from_idle() {
        assert_ne!(art_for(true, true), art_for(false, true));
        assert_ne!(art_for(true, false), art_for(false, false));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn windows_uses_white_glyph_on_dark_taskbar() {
        let dark = Image::from_bytes(art_for(false, true)).unwrap();
        let light = Image::from_bytes(art_for(false, false)).unwrap();
        let brightest = |image: &Image| {
            image
                .rgba()
                .chunks(4)
                .filter(|px| px[3] > 200)
                .map(|px| px[0])
                .max()
                .unwrap_or(0)
        };
        assert_eq!(brightest(&dark), 255);
        assert_eq!(brightest(&light), 0);
    }

    #[test]
    fn high_contrast_picks_the_glyph_from_the_theme_background() {
        // "Night sky" paints a black taskbar even though SystemUsesLightTheme may say light.
        assert!(dark_bar(true, Some((0, 0, 0)), Some(0)));
        assert!(dark_bar(true, Some((22, 22, 22)), Some(0)));
        // "Desert" paints a light taskbar.
        assert!(!dark_bar(true, Some((255, 255, 255)), Some(1)));
        assert!(!dark_bar(true, Some((222, 214, 190)), Some(0)));
        // Without a readable colour the safe choice is the white glyph.
        assert!(dark_bar(true, None, Some(1)));
    }

    #[test]
    fn without_high_contrast_the_system_light_theme_decides() {
        assert!(dark_bar(false, Some((255, 255, 255)), None));
        assert!(dark_bar(false, Some((255, 255, 255)), Some(0)));
        assert!(!dark_bar(false, Some((0, 0, 0)), Some(1)));
    }
}
