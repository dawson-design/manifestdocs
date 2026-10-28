//! References to a GitHub host, repo or issue.

use std::fmt;

/// A GitHub or GitHub Enterprise host name, such as `github.com`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host(String);

/// A repo as `owner/name`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepoRef {
    owner: String,
    name: String,
}

/// An issue as `owner/name#number`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IssueRef {
    repo: RepoRef,
    number: u64,
}

impl Host {
    pub fn parse(text: &str) -> Option<Self> {
        let allowed = text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'));
        let trimmed = !text.starts_with(['.', '-']) && !text.ends_with(['.', '-']);
        (!text.is_empty() && allowed && trimmed).then(|| Self(text.to_owned()))
    }
}

impl RepoRef {
    pub fn parse(text: &str) -> Option<Self> {
        let (owner, name) = text.split_once('/')?;
        (is_owner(owner) && is_repo_name(name)).then(|| Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }
}

impl IssueRef {
    pub fn parse(text: &str) -> Option<Self> {
        let (repo, number) = text.split_once('#')?;
        // u64 parsing accepts a leading `+`, which an issue number never has.
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let number = number.parse::<u64>().ok().filter(|n| *n > 0)?;
        Some(Self {
            repo: RepoRef::parse(repo)?,
            number,
        })
    }
}

// GitHub owners are letters, digits and single inner hyphens.
fn is_owner(text: &str) -> bool {
    let allowed = text
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    allowed && !text.is_empty() && !text.starts_with('-') && !text.ends_with('-')
}

fn is_repo_name(text: &str) -> bool {
    let allowed = text
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    allowed && !matches!(text, "" | "." | "..")
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for RepoRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

impl fmt::Display for IssueRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.repo, self.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repos_as_owner_and_name() {
        for text in ["acme/cookbook-docs", "example-org/cookbook.docs", "a/b_c"] {
            assert_eq!(
                RepoRef::parse(text).map(|r| r.to_string()),
                Some(text.into()),
                "{text}"
            );
        }
        for text in [
            "manifestdocs",
            "/manifestdocs",
            "-org/repo",
            "org-/repo",
            "org/..",
            "org/a/b",
            "org/a b",
        ] {
            assert_eq!(RepoRef::parse(text), None, "{text}");
        }
    }

    #[test]
    fn parses_issues_with_a_positive_number() {
        assert_eq!(
            IssueRef::parse("example-org/cookbook-docs#41").map(|i| i.to_string()),
            Some("example-org/cookbook-docs#41".into())
        );
        for text in [
            "example-org/cookbook-docs",
            "example-org/cookbook-docs#",
            "example-org/cookbook-docs#0",
            "example-org/cookbook-docs#+4",
            "#41",
        ] {
            assert_eq!(IssueRef::parse(text), None, "{text}");
        }
    }

    #[test]
    fn parses_host_names() {
        assert!(Host::parse("github.com").is_some());
        assert!(Host::parse("ghe.example.com").is_some());
        for text in ["", "https://github.com", ".github.com", "github.com/"] {
            assert_eq!(Host::parse(text), None, "{text}");
        }
    }
}
