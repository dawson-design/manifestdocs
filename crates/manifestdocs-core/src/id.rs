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

    /// A new ID from 3 random bytes. The I/O side supplies the bytes, so this stays pure.
    pub fn from_bytes(bytes: [u8; 3]) -> Self {
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        Self(format!("fd-{hex}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A node ID or the start of one, as typed on the command line: `fd-` and 4 to 6 lowercase
/// hex characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeIdPrefix(String);

impl NodeIdPrefix {
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix("fd-")?;
        let lowercase_hex = hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        ((4..=6).contains(&hex.len()) && lowercase_hex).then(|| Self(text.to_owned()))
    }

    pub fn matches(&self, id: &NodeId) -> bool {
        id.0.starts_with(&self.0)
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

    #[test]
    fn formats_random_bytes_as_a_valid_id() {
        let id = NodeId::from_bytes([0x0a, 0x9f, 0x00]);

        assert_eq!(id.as_str(), "fd-0a9f00");
        assert_eq!(NodeId::parse(id.as_str()), Some(id));
    }

    #[test]
    fn accepts_prefixes_of_four_to_six_hex_characters() {
        let id = NodeId::parse("fd-622eed").unwrap();
        for text in ["fd-622e", "fd-622ee", "fd-622eed"] {
            let prefix = NodeIdPrefix::parse(text).unwrap();
            assert!(prefix.matches(&id), "{text}");
        }
        assert!(!NodeIdPrefix::parse("fd-622f").unwrap().matches(&id));

        for text in ["fd-622", "fd-622eed0", "fd-622E", "622eed", "fd-62ze"] {
            assert_eq!(NodeIdPrefix::parse(text), None, "{text}");
        }
    }
}
