#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDomSnapshot {
    pub url: String,
    pub title: String,
    pub visible_text: String,
    pub selected_text: Option<String>,
    pub anchors: Vec<DomAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomAnchor {
    pub text: String,
    pub selector: Option<String>,
    pub scroll_y: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserAdapterNeed {
    NotBrowser,
    NotNeeded,
    Needed(&'static str),
}

/// Returns true when the captured surface is backed by a browser.
///
/// A valid web URL is the strongest signal because wrapper browsers do not all
/// advertise a conventional browser application name. App and bundle identity
/// remain useful for new-tab pages and other browser chrome where no URL is
/// available yet.
pub fn is_browser_surface(
    app_name: Option<&str>,
    bundle_id: Option<&str>,
    browser_url: Option<&str>,
) -> bool {
    browser_url.is_some_and(is_http_url)
        || [app_name, bundle_id]
            .into_iter()
            .flatten()
            .any(is_browser_identity)
}

pub fn is_http_url(value: &str) -> bool {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    let Some(remainder) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    remainder
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|host| !host.trim().is_empty())
}

fn is_browser_identity(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    [
        "browser",
        "safari",
        "chrome",
        "chromium",
        "firefox",
        "arc",
        "brave",
        "microsoft edge",
        "vivaldi",
        "opera",
        "helium",
        "zen",
        "orion",
        "dia",
        "sigmaos",
        "duckduckgo",
        "floorp",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

pub fn should_request_dom_adapter(
    is_browser_surface: bool,
    page_body_units: usize,
    chrome_heavy: bool,
) -> BrowserAdapterNeed {
    if !is_browser_surface {
        BrowserAdapterNeed::NotBrowser
    } else if chrome_heavy || page_body_units == 0 {
        BrowserAdapterNeed::Needed("native AX/OCR lacks page-body anchors")
    } else {
        BrowserAdapterNeed::NotNeeded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_without_body_requests_dom_enrichment() {
        assert_eq!(
            should_request_dom_adapter(true, 0, false),
            BrowserAdapterNeed::Needed("native AX/OCR lacks page-body anchors")
        );
    }

    #[test]
    fn non_browser_does_not_request_dom() {
        assert_eq!(
            should_request_dom_adapter(false, 0, true),
            BrowserAdapterNeed::NotBrowser
        );
    }

    #[test]
    fn recognizes_named_browsers_and_wrappers_without_urls() {
        for (app, bundle) in [
            ("Safari", "com.apple.Safari"),
            ("Google Chrome", "com.google.Chrome"),
            ("Firefox", "org.mozilla.firefox"),
            ("Arc", "company.thebrowser.Browser"),
            ("Helium", "net.imput.helium"),
        ] {
            assert!(is_browser_surface(Some(app), Some(bundle), None));
        }
    }

    #[test]
    fn valid_web_url_recognizes_unknown_wrapper() {
        assert!(is_browser_surface(
            Some("New Web Shell"),
            Some("example.wrapper"),
            Some("https://x.com/home")
        ));
        assert!(is_browser_surface(
            Some("Development WebView"),
            Some("example.wrapper"),
            Some("http://localhost:1420")
        ));
        assert!(!is_browser_surface(
            Some("Notes"),
            Some("com.apple.Notes"),
            Some("notes://local/item")
        ));
    }
}
