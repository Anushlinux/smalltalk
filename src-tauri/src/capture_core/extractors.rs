#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextModality {
    Accessibility,
    Ocr,
    Hybrid,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionDecision {
    pub text_modality: TextModality,
    pub run_ocr: bool,
    pub reason: &'static str,
}

pub fn decide_text_extraction(
    ax_text_chars: usize,
    ax_is_thin: bool,
    browser_chrome_heavy: bool,
    model_anchor_needed: bool,
) -> ExtractionDecision {
    if ax_text_chars == 0 {
        return ExtractionDecision {
            text_modality: TextModality::Ocr,
            run_ocr: true,
            reason: "accessibility returned no text",
        };
    }
    if ax_is_thin || browser_chrome_heavy {
        return ExtractionDecision {
            text_modality: TextModality::Hybrid,
            run_ocr: true,
            reason: "accessibility text is thin or chrome-heavy",
        };
    }
    if model_anchor_needed && ax_text_chars < 120 {
        return ExtractionDecision {
            text_modality: TextModality::Hybrid,
            run_ocr: true,
            reason: "model-safe line anchor needs visual enrichment",
        };
    }
    ExtractionDecision {
        text_modality: TextModality::Accessibility,
        run_ocr: false,
        reason: "accessibility text is sufficient",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ax_runs_ocr() {
        let decision = decide_text_extraction(0, false, false, false);

        assert!(decision.run_ocr);
        assert_eq!(decision.text_modality, TextModality::Ocr);
    }

    #[test]
    fn chrome_heavy_ax_runs_hybrid_ocr() {
        let decision = decide_text_extraction(800, false, true, false);

        assert!(decision.run_ocr);
        assert_eq!(decision.text_modality, TextModality::Hybrid);
    }

    #[test]
    fn strong_ax_skips_ocr() {
        let decision = decide_text_extraction(1_000, false, false, false);

        assert!(!decision.run_ocr);
        assert_eq!(decision.text_modality, TextModality::Accessibility);
    }
}
