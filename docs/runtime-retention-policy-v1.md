# Runtime Retention Policy v1

Policy identifier: `smalltalk.retention.v1`.

Smalltalk keeps enough recent evidence to explain and safely open the current Continue answer. It does not keep every historical row merely because an old derived result once referenced it. Runtime maintenance runs in small transactions and preserves current causal references.

| Data class | Runtime budget | Why it exists | Protection rule |
| --- | ---: | --- | --- |
| Low-value UI events | 5,000 rows | Recent scroll and noisy Accessibility context | Events referenced by triggers, transitions, typing bursts, or task actions are retained |
| High-value UI events | 20,000 rows | App, window, navigation, click, commit, error, and task-transition evidence | The same causal-reference rule applies |
| Capture triggers | 5,000 unbound rows | Explains why heavy capture was requested | Triggers bound to frames or typing bursts remain |
| Typing bursts | 5,000 unbound rows | Privacy-safe editing metadata and commit boundaries | Bursts bound to pre/post frames remain |
| Frames and image assets | 400 low-value duplicate frames and seven days | Visual evidence and exact return targets | Manual evidence and frames referenced by current Continue objects remain |
| OCR rows and spans | Follow their frame | Searchable and attributable screen text | Deleted only with an unprotected parent frame |
| Accessibility nodes | Follow their frame | Structural surface evidence | Deleted only with an unprotected parent frame |
| Content units | Follow their frame | Normalized task evidence | Deleted only with an unprotected parent frame |
| Window snapshots | Follow their frame | Window identity and layout | Deleted only with an unprotected parent frame |
| Derived Continue rows | Current rebuild plus bounded history | Continue ranking and explanation | Current task state, feedback, open events, and active evidence protect their source rows |
| Decision history | Latest 100 and at least 24 hours | Recent product auditability | Current/recent decisions remain; older refresh history is pruned |

Automatic maintenance is considered after every 250 persisted events, every 12 stored frames, and at capture-session stop. Runs are separated by at least 30 seconds on the event path. Each class deletes at most 250 rows per transaction. Repeated runs are safe and converge without pausing every event.

Manual cleanup remains dry-run first. A real cleanup must be explicitly requested. `VACUUM` is never automatic; it is performed only when the explicit cleanup input requests it.
