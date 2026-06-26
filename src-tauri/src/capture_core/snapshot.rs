use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotKind {
    FullDisplay,
    ActiveWindow,
    MetadataOnly,
}

#[derive(Debug, Clone)]
pub struct StagedSnapshot {
    pub kind: SnapshotKind,
    pub temp_path: Option<PathBuf>,
    pub final_path: Option<PathBuf>,
    pub byte_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotAcceptance {
    pub persist_image: bool,
    pub persist_frame: bool,
    pub reason: &'static str,
}

pub trait SnapshotProvider {
    fn provider_id(&self) -> &'static str;
    fn supports_active_window(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct ScreencaptureCliProvider;

impl SnapshotProvider for ScreencaptureCliProvider {
    fn provider_id(&self) -> &'static str {
        "screencapture_cli"
    }

    fn supports_active_window(&self) -> bool {
        true
    }
}

#[derive(Debug, Default)]
pub struct ScreenCaptureKitProvider;

impl SnapshotProvider for ScreenCaptureKitProvider {
    fn provider_id(&self) -> &'static str {
        "screencapturekit"
    }

    fn supports_active_window(&self) -> bool {
        true
    }
}

pub fn decide_snapshot_acceptance(
    model_eligible: bool,
    frame_worthy: bool,
    privacy_excluded: bool,
) -> SnapshotAcceptance {
    if privacy_excluded {
        return SnapshotAcceptance {
            persist_image: false,
            persist_frame: false,
            reason: "privacy excluded before image persistence",
        };
    }
    if !frame_worthy {
        return SnapshotAcceptance {
            persist_image: false,
            persist_frame: false,
            reason: "observation did not earn an image-backed frame",
        };
    }
    SnapshotAcceptance {
        persist_image: model_eligible,
        persist_frame: true,
        reason: if model_eligible {
            "accepted as model-safe evidence frame"
        } else {
            "accepted locally without model image"
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_excluded_snapshot_never_persists() {
        let decision = decide_snapshot_acceptance(true, true, true);

        assert!(!decision.persist_image);
        assert!(!decision.persist_frame);
    }

    #[test]
    fn metadata_only_observation_skips_image_frame() {
        let decision = decide_snapshot_acceptance(true, false, false);

        assert!(!decision.persist_image);
        assert!(!decision.persist_frame);
    }

    #[test]
    fn cli_provider_is_available_as_fallback_provider() {
        let provider = ScreencaptureCliProvider;

        assert_eq!(provider.provider_id(), "screencapture_cli");
        assert!(provider.supports_active_window());
    }
}
