#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyGapSummary {
    pub count: usize,
    pub frame_ranges: Vec<String>,
    pub message: Option<String>,
}

pub fn summarize_privacy_exclusions(frame_ids: &[i64]) -> PrivacyGapSummary {
    if frame_ids.is_empty() {
        return PrivacyGapSummary {
            count: 0,
            frame_ranges: Vec::new(),
            message: None,
        };
    }

    let mut ids = frame_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();

    let mut ranges = Vec::new();
    let mut start = ids[0];
    let mut previous = ids[0];
    for id in ids.into_iter().skip(1) {
        if id == previous + 1 {
            previous = id;
            continue;
        }
        ranges.push(format_range(start, previous));
        start = id;
        previous = id;
    }
    ranges.push(format_range(start, previous));

    let count = frame_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>()
        .len();
    let visible_ranges = ranges.iter().take(8).cloned().collect::<Vec<_>>();
    PrivacyGapSummary {
        count,
        frame_ranges: visible_ranges.clone(),
        message: Some(format!(
            "{} privacy-excluded frames omitted from model evidence ({})",
            count,
            visible_ranges.join(", ")
        )),
    }
}

fn format_range(start: i64, end: i64) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{}-{}", start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_exclusions_collapse_into_ranges() {
        let summary = summarize_privacy_exclusions(&[10, 11, 12, 20, 22, 23, 23]);

        assert_eq!(summary.count, 6);
        assert_eq!(summary.frame_ranges, vec!["10-12", "20", "22-23"]);
        assert!(summary
            .message
            .as_deref()
            .is_some_and(|message| message.contains("6 privacy-excluded")));
    }

    #[test]
    fn empty_privacy_summary_has_no_message() {
        let summary = summarize_privacy_exclusions(&[]);

        assert_eq!(summary.count, 0);
        assert!(summary.message.is_none());
    }
}
