//! The anchors GitHub gives Markdown headings.

use std::collections::{BTreeMap, BTreeSet};

/// The anchor for each heading, following GitHub's rules: lowercase, punctuation dropped,
/// spaces turned into hyphens, and `-1`, `-2` and so on added to repeats.
pub fn anchors<'a>(headings: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
    let mut occurrences: BTreeMap<String, usize> = BTreeMap::new();
    let mut anchors = BTreeSet::new();
    for heading in headings {
        let original = slug(heading);
        let mut anchor = original.clone();
        while occurrences.contains_key(&anchor) {
            let count = occurrences.entry(original.clone()).or_default();
            *count += 1;
            anchor = format!("{original}-{count}");
        }
        occurrences.insert(anchor.clone(), 0);
        anchors.insert(anchor);
    }
    anchors
}

// Letters and numbers in any script, hyphens and underscores stay; other punctuation goes.
fn slug(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}
