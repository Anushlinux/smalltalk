pub const RESUME_QUERY_SCHEMA_V2: &str = "smalltalk.resume_query.v2";
pub const DEFAULT_MAX_JSON_CHARS: u32 = 25_000;
pub const DEFAULT_MAX_MODEL_IMAGES: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeDossierPolicy {
    pub schema: &'static str,
    pub max_json_chars: u32,
    pub max_model_images: u32,
    pub max_episode_cards: u32,
}

impl Default for ResumeDossierPolicy {
    fn default() -> Self {
        Self {
            schema: RESUME_QUERY_SCHEMA_V2,
            max_json_chars: DEFAULT_MAX_JSON_CHARS,
            max_model_images: DEFAULT_MAX_MODEL_IMAGES,
            max_episode_cards: 8,
        }
    }
}

pub fn bounded_json_chars(requested: Option<u32>) -> i64 {
    requested
        .unwrap_or(DEFAULT_MAX_JSON_CHARS)
        .clamp(5_000, DEFAULT_MAX_JSON_CHARS) as i64
}

pub fn bounded_model_images(requested: Option<u32>) -> usize {
    requested
        .unwrap_or(DEFAULT_MAX_MODEL_IMAGES)
        .clamp(1, DEFAULT_MAX_MODEL_IMAGES) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_limits_default_to_v2_policy() {
        let policy = ResumeDossierPolicy::default();

        assert_eq!(policy.schema, "smalltalk.resume_query.v2");
        assert_eq!(policy.max_json_chars, 25_000);
        assert_eq!(policy.max_model_images, 4);
    }

    #[test]
    fn requested_limits_are_capped() {
        assert_eq!(bounded_json_chars(Some(80_000)), 25_000);
        assert_eq!(bounded_model_images(Some(8)), 4);
    }
}
