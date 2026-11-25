//! The few strings the Rust side shows itself (tray menu, window titles), in the UI language of the
//! profile (FR-019). Everything else is translated by the webview catalogues.

use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    TranslateSelection,
    TranslateScreen,
    SpeakSelection,
    QuickInput,
    Settings,
    About,
    ReportProblem,
    Quit,
    UpdateAvailable,
    TooltipBusy,
    SettingsTitle,
    WelcomeTitle,
}

/// `{version}` in `UpdateAvailable` is filled by the caller.
pub fn text(ui_language: &str, key: Text) -> &'static str {
    let vi = ui_language == "vi";
    match key {
        Text::TranslateSelection => pick(vi, "Translate Selection", "Dịch đoạn đã chọn"),
        Text::TranslateScreen => pick(vi, "Translate Screen Region", "Dịch vùng màn hình"),
        Text::SpeakSelection => pick(vi, "Speak Selection", "Đọc đoạn đã chọn"),
        Text::QuickInput => pick(vi, "Quick Input…", "Nhập nhanh…"),
        Text::Settings => pick(vi, "Settings…", "Cài đặt…"),
        Text::About => pick(vi, "About Verso", "Giới thiệu Verso"),
        Text::ReportProblem => pick(vi, "Report a problem…", "Báo lỗi…"),
        Text::Quit => pick(vi, "Quit Verso", "Thoát Verso"),
        Text::UpdateAvailable => pick(
            vi,
            "Update available — v{version}",
            "Có bản cập nhật — v{version}",
        ),
        Text::TooltipBusy => pick(vi, "Verso — translating…", "Verso — đang dịch…"),
        Text::SettingsTitle => pick(vi, "Verso Settings", "Cài đặt Verso"),
        Text::WelcomeTitle => pick(vi, "Welcome to Verso", "Chào mừng đến với Verso"),
    }
}

fn pick(vi: bool, english: &'static str, vietnamese: &'static str) -> &'static str {
    if vi {
        vietnamese
    } else {
        english
    }
}

/// The string in the current profile's UI language.
pub fn current(app: &AppHandle, key: Text) -> &'static str {
    let language = app
        .state::<crate::commands::AppState>()
        .settings
        .get()
        .ui_language;
    text(&language, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vietnamese_differs_and_english_is_the_fallback() {
        assert_eq!(text("vi", Text::Quit), "Thoát Verso");
        assert_eq!(text("en", Text::Quit), "Quit Verso");
        assert_eq!(text("fr", Text::Quit), "Quit Verso");
        assert!(text("vi", Text::UpdateAvailable).contains("{version}"));
        assert!(text("en", Text::UpdateAvailable).contains("{version}"));
    }
}
