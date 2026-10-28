//! Node IDs.

use std::fmt;

/// A node ID: `fd-` and exactly 6 lowercase hex characters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(String);

impl NodeId {
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix("fd-")?;
        let lowercase_hex = hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        (hex.len() == 6 && lowercase_hex).then(|| Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_fd_and_six_lowercase_hex_characters() {
        assert!(NodeId::parse("fd-622eed").is_some());
        assert!(NodeId::parse("fd-0a9f00").is_some());

        let rejected = [
            "fd-622EED",
            "FD-622eed",
            "fd-622ee",
            "fd-622eed0",
            "fd-62zeed",
            "622eed",
            "dc-622eed",
            "",
        ];
        for text in rejected {
            assert_eq!(NodeId::parse(text), None, "{text}");
        }
    }
}
