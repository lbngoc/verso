//! Every fixed link the app opens points at the public repository, the one users can reach: it holds
//! the releases, the issue form and the user documents synced from `docs/public/`. Change the
//! repository here; the updater endpoint in `tauri.conf.json` is the only other copy.
//!
//! The webview asks for a page by name (`open_public_page`), never by URL.

/// The public repository. A macro, so `concat!` can build the other constants from it.
#[macro_export]
macro_rules! public_repo {
    () => {
        "https://github.com/lbngoc/verso-translate"
    };
}

pub const REPO_URL: &str = public_repo!();
pub const ISSUE_URL: &str = concat!(public_repo!(), "/issues/new");
pub const TERMS_URL: &str = concat!(public_repo!(), "/blob/main/TERMS.md");
pub const PRIVACY_URL: &str = concat!(public_repo!(), "/blob/main/PRIVACY.md");
pub const THIRD_PARTY_URL: &str = concat!(public_repo!(), "/blob/main/THIRD_PARTY.md");

/// The pages the About section links to.
pub fn page_url(page: &str) -> Option<&'static str> {
    match page {
        "home" => Some(REPO_URL),
        "terms" => Some(TERMS_URL),
        "privacy" => Some(PRIVACY_URL),
        "licences" => Some(THIRD_PARTY_URL),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_is_in_the_public_repository() {
        for page in ["home", "terms", "privacy", "licences"] {
            let url = page_url(page).expect(page);
            assert!(url.starts_with(REPO_URL), "{url}");
        }
        assert!(ISSUE_URL.starts_with(REPO_URL));
        assert_eq!(page_url("anything-else"), None);
    }

    #[test]
    fn the_updater_endpoint_uses_the_same_repository() {
        let config = include_str!("../tauri.conf.json");
        assert!(
            config.contains(concat!(
                public_repo!(),
                "/releases/latest/download/latest.json"
            )),
            "tauri.conf.json updater endpoint must follow links::REPO_URL"
        );
    }
}
