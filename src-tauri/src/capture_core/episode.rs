#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeBoundary {
    pub surface_key: String,
    pub reason: &'static str,
}

pub fn surface_key(app: &str, title: &str, url_or_doc: Option<&str>) -> String {
    let primary = url_or_doc.unwrap_or(title).trim().to_lowercase();
    format!("{}::{}", app.trim().to_lowercase(), primary)
}

pub fn episode_boundary(previous_key: Option<&str>, next_key: &str) -> Option<EpisodeBoundary> {
    match previous_key {
        Some(previous) if previous == next_key => None,
        _ => Some(EpisodeBoundary {
            surface_key: next_key.to_string(),
            reason: "surface identity changed",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_surface_does_not_create_episode_boundary() {
        let key = surface_key("Codex", "capture.rs", None);

        assert!(episode_boundary(Some(&key), &key).is_none());
    }

    #[test]
    fn url_identity_wins_over_title() {
        assert_eq!(
            surface_key("Helium", "ChatGPT", Some("https://chatgpt.com/c/1")),
            "helium::https://chatgpt.com/c/1"
        );
    }
}
