//! The few words the components show themselves, such as tooltips on
//! their own buttons. English until the app sets its translations.

use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Labels {
    pub close: String,
    pub more: String,
    pub keep_toolbar_shown: String,
    pub auto_hide_toolbar: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            close: "Close".to_owned(),
            more: "More".to_owned(),
            keep_toolbar_shown: "Keep toolbar shown".to_owned(),
            auto_hide_toolbar: "Hide toolbar automatically".to_owned(),
        }
    }
}

static LABELS: RwLock<Option<Labels>> = RwLock::new(None);

/// Replaces the labels, after the app loads or changes its language.
pub fn set(labels: Labels) {
    if let Ok(mut current) = LABELS.write() {
        *current = Some(labels);
    }
}

pub fn get() -> Labels {
    LABELS
        .read()
        .ok()
        .and_then(|labels| labels.clone())
        .unwrap_or_default()
}
