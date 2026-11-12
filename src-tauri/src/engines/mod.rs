pub mod gtx_parse;
pub mod http_bing;
pub mod http_google;

/// Browser User-Agent sent by both keyless engines. A bare `Mozilla/5.0` gets `401` from Bing's
/// `/ttranslatev3` (verified 2026-11-09, 3/3), while this full string, translate-shell's default
/// (`include/Main.awk`, Unlicense), gets `200`.
pub const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
     AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36 Edg/104.0.1293.54";
