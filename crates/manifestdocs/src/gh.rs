//! Reads GitHub issues through the `gh` CLI. It only reads: the binary never writes to GitHub.

use std::fmt;
use std::io;
use std::process::Command;

use manifestdocs_core::refs::{Host, IssueRef, RepoRef};
use manifestdocs_core::work::IssueBody;
use serde_json::Value;

#[derive(Debug)]
pub enum GhError {
    Missing,
    Unauthenticated { host: Host },
    Failed(String),
}

/// Whether `gh` can read one issue.
#[derive(Debug, PartialEq, Eq)]
pub enum Lookup {
    Found,
    /// The issue, or its repo, doesn't exist or can't be read.
    NotFound,
    Failed(String),
}

/// How many issues one existence query asks about.
const BATCH: usize = 100;

/// `gh` exits with 4 when a command needs a login it doesn't have.
const AUTH_REQUIRED: i32 = 4;

const SEARCH_QUERY: &str = "query($q: String!, $cursor: String) { \
    search(query: $q, type: ISSUE, first: 100, after: $cursor) { \
    issueCount pageInfo { hasNextPage endCursor } nodes { ... on Issue { number body } } } }";

/// Asks whether each of `numbers` is an issue in `repo` that `gh` can read, in batches of
/// up to 100 issues per query. An error fails the whole repo.
pub fn lookup_issues(
    host: &Host,
    repo: &RepoRef,
    numbers: &[u64],
) -> Result<Vec<(u64, Lookup)>, GhError> {
    let mut lookups = Vec::new();
    for batch in numbers.chunks(BATCH) {
        let query = existence_query(batch);
        let response = graphql(host, &repo_fields(&query, repo))?;
        lookups.extend(read_lookups(&response, batch).map_err(GhError::Failed)?);
    }
    Ok(lookups)
}

// Each issue gets an alias, such as `i41`, so one query can ask about many.
fn existence_query(numbers: &[u64]) -> String {
    let issues: Vec<String> = numbers
        .iter()
        .map(|number| format!("i{number}: issue(number: {number}) {{ number }}"))
        .collect();
    format!(
        "query($owner: String!, $name: String!) {{ repository(owner: $owner, name: $name) {{ {} }} }}",
        issues.join(" ")
    )
}

fn read_lookups(response: &Value, numbers: &[u64]) -> Result<Vec<(u64, Lookup)>, String> {
    let repository = response.pointer("/data/repository").unwrap_or(&Value::Null);
    if repository.is_null() {
        return match error_at(response, &["repository"]) {
            Some((true, _)) => Ok(numbers.iter().map(|n| (*n, Lookup::NotFound)).collect()),
            Some((false, message)) => Err(message),
            None => Err(first_error(response)),
        };
    }
    let lookup = |number: u64| {
        let alias = format!("i{number}");
        if repository.get(&alias).is_some_and(Value::is_object) {
            return Lookup::Found;
        }
        match error_at(response, &["repository", &alias]) {
            Some((true, _)) => Lookup::NotFound,
            Some((false, message)) => Lookup::Failed(message),
            None => Lookup::Failed("no answer from `gh`".into()),
        }
    };
    Ok(numbers.iter().map(|n| (*n, lookup(*n))).collect())
}

/// Whether the GraphQL error at `path` is NOT_FOUND, and its message.
fn error_at(response: &Value, path: &[&str]) -> Option<(bool, String)> {
    let errors = response.get("errors")?.as_array()?;
    let error = errors.iter().find(|error| {
        let at = error.get("path").and_then(Value::as_array);
        at.is_some_and(|at| {
            at.iter()
                .map(Value::as_str)
                .eq(path.iter().map(|p| Some(*p)))
        })
    })?;
    let not_found = error.get("type").and_then(Value::as_str) == Some("NOT_FOUND");
    Some((not_found, message(error)))
}

fn first_error(response: &Value) -> String {
    let error = response.pointer("/errors/0");
    error.map_or_else(|| "no answer from `gh`".into(), message)
}

fn message(error: &Value) -> String {
    let message = error.get("message").and_then(Value::as_str);
    message.unwrap_or("unknown GraphQL error").to_owned()
}

/// What a search of one repo found.
#[derive(Debug, Default)]
pub struct SearchResults {
    pub issues: Vec<IssueBody>,
    /// How many results were read. GitHub search returns at most 1,000.
    pub read: u64,
    /// How many results GitHub counted.
    pub total: u64,
}

/// Every issue in `repo` whose body has the phrase `feature: fd-`, read page by page, oldest
/// first so pages don't shift while they're read. Search ignores the punctuation, so the
/// caller parses each body for the `feature:` line itself.
pub fn search_issues(host: &Host, repo: &RepoRef) -> Result<SearchResults, GhError> {
    let q = format!("repo:{repo} is:issue in:body \"feature: fd-\" sort:created-asc");
    let mut results = SearchResults::default();
    let mut cursor: Option<String> = None;
    loop {
        let mut fields = vec![("query", SEARCH_QUERY), ("q", q.as_str())];
        fields.extend(cursor.as_deref().map(|cursor| ("cursor", cursor)));
        let response = graphql(host, &fields)?;
        let page = search_page(&response).map_err(GhError::Failed)?;
        let nodes = page.get("nodes").and_then(Value::as_array);
        let nodes = nodes.map_or(&[][..], Vec::as_slice);
        results.read += nodes.len() as u64;
        results.total = page.get("issueCount").and_then(Value::as_u64).unwrap_or(0);
        results.issues.extend(issue_bodies(repo, nodes));
        cursor = next_page(page);
        if cursor.is_none() {
            return Ok(results);
        }
    }
}

// Any error fails the page, even next to a partial `search`, since a missing result can't be
// told apart from no result.
fn search_page(response: &Value) -> Result<&Value, String> {
    let errors = response.get("errors").and_then(Value::as_array);
    if errors.is_some_and(|errors| !errors.is_empty()) {
        return Err(first_error(response));
    }
    match response.pointer("/data/search") {
        Some(search) if search.is_object() => Ok(search),
        _ => Err(first_error(response)),
    }
}

// Search also finds pull requests, which come back as empty objects and are left out.
fn issue_bodies(repo: &RepoRef, nodes: &[Value]) -> Vec<IssueBody> {
    nodes
        .iter()
        .filter_map(|node| {
            let number = node.get("number")?.as_u64()?;
            Some(IssueBody {
                issue: IssueRef::parse(&format!("{repo}#{number}"))?,
                body: node.get("body")?.as_str()?.to_owned(),
            })
        })
        .collect()
}

fn next_page(search: &Value) -> Option<String> {
    let more = search.pointer("/pageInfo/hasNextPage")?.as_bool()?;
    let cursor = search.pointer("/pageInfo/endCursor")?.as_str()?;
    more.then(|| cursor.to_owned())
}

fn repo_fields<'a>(query: &'a str, repo: &'a RepoRef) -> Vec<(&'a str, &'a str)> {
    vec![
        ("query", query),
        ("owner", repo.owner()),
        ("name", repo.name()),
    ]
}

/// Runs a read-only `gh api graphql` query. Each field is passed with `-f`, as a string, so
/// an owner that looks like a number stays text. `gh` exits 1 when the answer has GraphQL
/// errors, so the answer is read whatever the exit code.
fn graphql(host: &Host, fields: &[(&str, &str)]) -> Result<Value, GhError> {
    let mut command = Command::new("gh");
    command.args(["api", "graphql", "--hostname", &host.to_string()]);
    for (key, value) in fields {
        command.args(["-f", &format!("{key}={value}")]);
    }
    let output = command.output().map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => GhError::Missing,
        _ => GhError::Failed(format!("can't run `gh`: {error}")),
    })?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    // A host with no stored token gets an anonymous request, which GitHub refuses with 401.
    if output.status.code() == Some(AUTH_REQUIRED) || stderr.contains("HTTP 401") {
        return Err(GhError::Unauthenticated { host: host.clone() });
    }
    serde_json::from_slice(&output.stdout).map_err(|_| match stderr.is_empty() {
        true => GhError::Failed(format!("`gh` exited with {}", output.status)),
        false => GhError::Failed(stderr),
    })
}

impl fmt::Display for GhError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str(
                "`gh` isn't installed; install the GitHub CLI from https://cli.github.com",
            ),
            Self::Unauthenticated { host } => write!(
                f,
                "`gh` isn't logged in to {host}; run `gh auth login --hostname {host}`"
            ),
            Self::Failed(detail) => write!(f, "`gh` failed: {detail}"),
        }
    }
}

impl std::error::Error for GhError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookups(response: &str, numbers: &[u64]) -> Result<Vec<(u64, Lookup)>, String> {
        read_lookups(&serde_json::from_str(response).unwrap(), numbers)
    }

    #[test]
    fn reads_each_issue_in_a_batch_on_its_own() {
        let response = r#"{"data":{"repository":{"i1":{"number":1},"i2":null,"i3":null}},
            "errors":[{"type":"NOT_FOUND","path":["repository","i2"],"message":"gone"},
                      {"type":"FORBIDDEN","path":["repository","i3"],"message":"no access"}]}"#;
        assert_eq!(
            lookups(response, &[1, 2, 3]),
            Ok(vec![
                (1, Lookup::Found),
                (2, Lookup::NotFound),
                (3, Lookup::Failed("no access".into()))
            ])
        );
    }

    #[test]
    fn reads_a_repo_that_doesnt_exist_as_every_issue_not_found() {
        let response = r#"{"data":{"repository":null},
            "errors":[{"type":"NOT_FOUND","path":["repository"],"message":"no repo"}]}"#;
        assert_eq!(
            lookups(response, &[1, 2]),
            Ok(vec![(1, Lookup::NotFound), (2, Lookup::NotFound)])
        );
    }

    #[test]
    fn fails_the_batch_when_the_answer_has_no_repository() {
        let response = r#"{"errors":[{"type":"RATE_LIMITED","message":"slow down"}]}"#;
        assert_eq!(lookups(response, &[1]), Err("slow down".into()));
    }

    #[test]
    fn fails_a_search_page_with_no_search_or_with_errors() {
        let page = |response: &str| {
            let response: Value = serde_json::from_str(response).unwrap();
            search_page(&response).cloned()
        };
        let null = r#"{"data":{"search":null},"errors":[{"message":"timed out"}]}"#;
        assert_eq!(page(null), Err("timed out".into()));
        let partial = r#"{"data":{"search":{"nodes":[]}},"errors":[{"message":"partial"}]}"#;
        assert_eq!(page(partial), Err("partial".into()));
        assert_eq!(
            page(r#"{"data":{"search":null}}"#),
            Err("no answer from `gh`".into())
        );
        let ok = r#"{"data":{"search":{"nodes":[]}}}"#;
        assert_eq!(page(ok), Ok(serde_json::json!({ "nodes": [] })));
    }
}
