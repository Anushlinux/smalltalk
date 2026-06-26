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
}
