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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_headings_the_way_github_does() {
        let cases = [
            ("Write path", "write-path"),
            ("A & B", "a--b"),
            ("Why `check` exits 1", "why-check-exits-1"),
            ("snake_case and kebab-case", "snake_case-and-kebab-case"),
            ("Q&A: what's next?", "qa-whats-next"),
            ("Café crème", "café-crème"),
            ("日本語の見出し", "日本語の見出し"),
            ("v1.0 (beta)", "v10-beta"),
        ];
        for (heading, expected) in cases {
            assert_eq!(slug(heading), expected, "{heading}");
        }
    }

    #[test]
    fn numbers_repeated_headings() {
        let anchors = anchors(["Notes", "Notes", "Notes-1", "Notes"]);

        let expected = ["notes", "notes-1", "notes-1-1", "notes-2"];
        assert_eq!(anchors, expected.map(String::from).into());
    }
}
