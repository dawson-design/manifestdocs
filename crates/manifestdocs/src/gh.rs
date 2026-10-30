//! Reads GitHub issues through the `gh` CLI. It only reads: the binary never writes to GitHub.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::process::Command;

use manifestdocs_core::refs::{Host, IssueRef, RepoRef};
use manifestdocs_core::work::{
    Fetched, Issue, IssueBody, IssueKey, IssueState, IssueText, Parents, SubIssue, Takeovers,
    Unreadable,
};
use serde_json::{Value, json};

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
    let mut seen = BTreeSet::new();
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
        match &cursor {
            None => return Ok(results),
            // A repeated cursor would read the same page forever.
            Some(next) if !seen.insert(next.clone()) => {
                return Err(GhError::Failed(format!(
                    "GitHub repeated the page cursor `{next}`"
                )));
            }
            Some(_) => {}
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

/// Which fields `fetch_hierarchy` reads for each issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fields {
    /// What `work` shows.
    Work,
    /// Also the body and creation time, which `stale` reads. A body can be 65 KB, so `work`
    /// doesn't ask for it.
    WithText,
}

/// The fields every issue is read with, and its first 100 sub-issues.
const ISSUE_FIELDS: &str = "number title state issueType { name } \
    parent { number repository { nameWithOwner } } \
    subIssues(first: 100) { pageInfo { hasNextPage endCursor } nodes { number repository { nameWithOwner } } }";

fn fragment(fields: Fields) -> String {
    let text = match fields {
        Fields::Work => "",
        Fields::WithText => "body createdAt ",
    };
    format!("fragment issue on Issue {{ {text}{ISSUE_FIELDS} }}")
}

/// Reads `issues` and their sub-issues at any depth, level by level until none are left: one
/// query per repo for each level, in batches of up to 100 issues. It reads an issue in
/// `takeovers` but not below it, since a deeper node takes its sub-issues. An issue `gh` can't
/// read is kept with the reason, and only a failure of a whole query is an error.
pub fn fetch_hierarchy(
    host: &Host,
    issues: &[IssueRef],
    takeovers: &Takeovers,
    fields: Fields,
) -> Result<Fetched, GhError> {
    let mut fetched = Fetched::new();
    let mut level = issues.to_vec();
    while !level.is_empty() {
        let mut next = Vec::new();
        for (repo, numbers) in by_repo(&level, |key| fetched.contains_key(key)) {
            for (reference, issue) in fetch_issues(host, &repo, &numbers, takeovers, fields)? {
                let key = IssueKey::from(&reference);
                if let (Ok(issue), false) = (&issue, takeovers.contains_key(&key)) {
                    next.extend(issue.sub_issues.iter().filter_map(|sub| match sub {
                        SubIssue::Issue(sub) => Some(sub.clone()),
                        SubIssue::Unreadable(_) => None,
                    }));
                }
                fetched.insert(key, issue);
            }
        }
        level = next;
    }
    Ok(fetched)
}

/// Reads the parent of each of `issues`, then each parent's parent, level by level up to
/// the top, batched as `fetch_hierarchy` batches. A parent `gh` can't read ends its chain.
pub fn fetch_parents(host: &Host, issues: &[IssueRef]) -> Result<Parents, GhError> {
    let mut parents = Parents::new();
    let mut level = issues.to_vec();
    while !level.is_empty() {
        let mut next = Vec::new();
        for (repo, numbers) in by_repo(&level, |key| parents.contains_key(key)) {
            for (issue, parent) in fetch_parent_level(host, &repo, &numbers)? {
                next.extend(parent.clone());
                parents.insert(IssueKey::from(&issue), parent);
            }
        }
        level = next;
    }
    Ok(parents)
}

/// The parent of each of `numbers` in `repo`, in batches of up to 100 issues.
fn fetch_parent_level(
    host: &Host,
    repo: &RepoRef,
    numbers: &[u64],
) -> Result<Vec<(IssueRef, Option<IssueRef>)>, GhError> {
    let mut found = Vec::new();
    for batch in numbers.chunks(BATCH) {
        let query = aliased_query(batch, "parent { number repository { nameWithOwner } }");
        let response = graphql(host, &repo_fields(&query, repo))?;
        let read = |value: &Value, _: &str| Some(value.get("parent").and_then(listed_issue));
        let answers = read_aliases(&response, batch, read).map_err(GhError::Failed)?;
        for (number, answer) in answers {
            let parent = match answer {
                Answer::Found(parent) => parent,
                Answer::NotFound | Answer::Failed(_) => None,
            };
            found.extend(issue_ref(repo, number).map(|issue| (issue, parent)));
        }
    }
    Ok(found)
}

/// The issues in `level` not yet `read`, by repo, with repo names compared without case.
fn by_repo(level: &[IssueRef], read: impl Fn(&IssueKey) -> bool) -> Vec<(RepoRef, Vec<u64>)> {
    let mut repos: BTreeMap<String, (RepoRef, BTreeSet<u64>)> = BTreeMap::new();
    for issue in level.iter().filter(|issue| !read(&IssueKey::from(*issue))) {
        let name = issue.repo().to_string().to_ascii_lowercase();
        let (_, numbers) = repos
            .entry(name)
            .or_insert_with(|| (issue.repo().clone(), BTreeSet::new()));
        numbers.insert(issue.number());
    }
    let repos = repos.into_values();
    repos
        .map(|(repo, numbers)| (repo, numbers.into_iter().collect()))
        .collect()
}

/// An issue, with the cursor after its first page of sub-issues when there are more.
struct Read(Issue, Option<String>);

/// An issue, or why `gh` couldn't read it.
type IssueOrWhy = Result<Issue, Unreadable>;

/// Reads each of `numbers` in `repo` with all its sub-issues, in batches of up to 100 issues.
fn fetch_issues(
    host: &Host,
    repo: &RepoRef,
    numbers: &[u64],
    takeovers: &Takeovers,
    fields: Fields,
) -> Result<Vec<(IssueRef, IssueOrWhy)>, GhError> {
    let mut issues = Vec::new();
    for batch in numbers.chunks(BATCH) {
        let query = aliased_query(batch, "...issue");
        let query = format!("{query} {}", fragment(fields));
        let response = graphql(host, &repo_fields(&query, repo))?;
        let read = |value: &Value, alias: &str| read_issue(value, alias, repo, &response);
        for (number, answer) in read_aliases(&response, batch, read).map_err(GhError::Failed)? {
            let taken = issue_ref(repo, number)
                .is_some_and(|issue| takeovers.contains_key(&IssueKey::from(&issue)));
            let issue = match answer {
                // A deeper node takes its sub-issues, so only its first fetch is kept.
                Answer::Found(read) if taken => Ok(read.0),
                Answer::Found(read) => Ok(all_sub_issues(host, repo, number, read)?),
                Answer::NotFound => Err(Unreadable::NotFound),
                Answer::Failed(detail) => Err(Unreadable::Failed(detail)),
            };
            issues.extend(issue_ref(repo, number).map(|reference| (reference, issue)));
        }
    }
    Ok(issues)
}

/// The issue with every page of its sub-issues.
fn all_sub_issues(host: &Host, repo: &RepoRef, number: u64, read: Read) -> Result<Issue, GhError> {
    let Read(mut issue, cursor) = read;
    if let Some(cursor) = cursor {
        let (more, repeated) = more_sub_issues(host, repo, number, cursor)?;
        issue.sub_issues.extend(more);
        if repeated {
            let after = issue.sub_issues.len();
            let unlisted = Unreadable::Unlisted { after };
            issue.sub_issues.push(SubIssue::Unreadable(unlisted));
        }
    }
    Ok(issue)
}

/// A query that asks for `fields` of each of `numbers`, as `i<number>`, in one repo.
fn aliased_query(numbers: &[u64], fields: &str) -> String {
    let issues: Vec<String> = numbers
        .iter()
        .map(|number| format!("i{number}: issue(number: {number}) {{ {fields} }}"))
        .collect();
    format!(
        "query($owner: String!, $name: String!) {{ repository(owner: $owner, name: $name) {{ {} }} }}",
        issues.join(" ")
    )
}

/// What `gh` answered for one issue in a batch.
enum Answer<T> {
    Found(T),
    /// The issue, or its repo, doesn't exist or can't be read.
    NotFound,
    Failed(String),
}

/// Each issue in a batch's answer, read with `read` from its value and alias. An error on the
/// repo, such as FORBIDDEN, is each issue's answer, and an error that isn't about the repo or
/// an issue fails the batch.
fn read_aliases<T>(
    response: &Value,
    numbers: &[u64],
    read: impl Fn(&Value, &str) -> Option<T>,
) -> Result<Vec<(u64, Answer<T>)>, String> {
    let repository = response.pointer("/data/repository").unwrap_or(&Value::Null);
    if repository.is_null() {
        let every =
            |answer: &dyn Fn() -> Answer<T>| numbers.iter().map(|n| (*n, answer())).collect();
        return match error_at(response, &["repository"]) {
            Some((true, _)) => Ok(every(&|| Answer::NotFound)),
            Some((false, message)) => Ok(every(&|| Answer::Failed(message.clone()))),
            None => Err(first_error(response)),
        };
    }
    let answer = |number: u64| {
        let alias = format!("i{number}");
        if let Some(value) = repository.get(&alias).filter(|value| value.is_object()) {
            return match read(value, &alias) {
                Some(found) => Answer::Found(found),
                None => Answer::Failed("unexpected answer from `gh`".into()),
            };
        }
        match error_at(response, &["repository", &alias]) {
            Some((true, _)) => Answer::NotFound,
            Some((false, message)) => Answer::Failed(message),
            None => Answer::Failed("no answer from `gh`".into()),
        }
    };
    Ok(numbers.iter().map(|n| (*n, answer(*n))).collect())
}

/// The issue at `alias` in `response`.
fn read_issue(value: &Value, alias: &str, repo: &RepoRef, response: &Value) -> Option<Read> {
    let state = match value.get("state")?.as_str()? {
        "OPEN" => IssueState::Open,
        "CLOSED" => IssueState::Closed,
        _ => return None,
    };
    let sub_issues = value.get("subIssues")?;
    let nodes = [
        json!("repository"),
        json!(alias),
        json!("subIssues"),
        json!("nodes"),
    ];
    let issue = Issue {
        reference: issue_ref(repo, value.get("number")?.as_u64()?)?,
        title: value.get("title")?.as_str()?.to_owned(),
        state,
        issue_type: value
            .pointer("/issueType/name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        parent: value.get("parent").and_then(listed_issue),
        sub_issues: listed_sub_issues(sub_issues, response, &nodes),
        text: issue_text(value),
    };
    Some(Read(issue, next_page(sub_issues)))
}

/// The body and creation time, when the query asked for them.
fn issue_text(value: &Value) -> Option<IssueText> {
    Some(IssueText {
        body: value.get("body")?.as_str()?.to_owned(),
        created_at: value.get("createdAt")?.as_str()?.to_owned(),
    })
}

/// An issue in a list, as `{ number repository { nameWithOwner } }`.
fn listed_issue(value: &Value) -> Option<IssueRef> {
    let repo = value.pointer("/repository/nameWithOwner")?.as_str()?;
    IssueRef::parse(&format!("{repo}#{}", value.get("number")?.as_u64()?))
}

/// One page of sub-issues, whose nodes are at `nodes` in `response`. A node GitHub couldn't
/// return is unreadable, with the error at its path.
fn listed_sub_issues(connection: &Value, response: &Value, nodes: &[Value]) -> Vec<SubIssue> {
    let list = connection.get("nodes").and_then(Value::as_array);
    let list = list.map_or(&[][..], Vec::as_slice);
    let sub_issue = |(index, node): (usize, &Value)| {
        if let Some(issue) = listed_issue(node) {
            return SubIssue::Issue(issue);
        }
        let path: Vec<Value> = nodes.iter().cloned().chain([json!(index)]).collect();
        let error = error_under(response, &path);
        let not_found =
            error.and_then(|e| e.get("type")).and_then(Value::as_str) == Some("NOT_FOUND");
        SubIssue::Unreadable(match (not_found, error) {
            (true, _) => Unreadable::NotFound,
            (false, Some(error)) => Unreadable::Failed(message(error)),
            (false, None) => Unreadable::Failed("no answer from `gh`".into()),
        })
    };
    list.iter().enumerate().map(sub_issue).collect()
}

/// The first GraphQL error at `path` or below it.
fn error_under<'a>(response: &'a Value, path: &[Value]) -> Option<&'a Value> {
    let errors = response.get("errors")?.as_array()?;
    errors.iter().find(|error| {
        let at = error.get("path").and_then(Value::as_array);
        at.is_some_and(|at| at.starts_with(path))
    })
}

fn issue_ref(repo: &RepoRef, number: u64) -> Option<IssueRef> {
    IssueRef::parse(&format!("{repo}#{number}"))
}

/// The rest of an issue's sub-issues, which GitHub returns 100 at a time, from `cursor` on,
/// and whether it stopped early because GitHub repeated a cursor, which would loop.
fn more_sub_issues(
    host: &Host,
    repo: &RepoRef,
    number: u64,
    cursor: String,
) -> Result<(Vec<SubIssue>, bool), GhError> {
    let query = format!(
        "query($owner: String!, $name: String!, $cursor: String!) {{ repository(owner: $owner, name: $name) {{ \
         i{number}: issue(number: {number}) {{ subIssues(first: 100, after: $cursor) {{ \
         pageInfo {{ hasNextPage endCursor }} nodes {{ number repository {{ nameWithOwner }} }} }} }} }} }}"
    );
    let nodes = [
        json!("repository"),
        json!(format!("i{number}")),
        json!("subIssues"),
        json!("nodes"),
    ];
    let mut seen = BTreeSet::from([cursor.clone()]);
    let mut sub_issues = Vec::new();
    let mut cursor = Some(cursor);
    while let Some(after) = cursor {
        let mut fields = repo_fields(&query, repo);
        fields.push(("cursor", &after));
        let response = graphql(host, &fields)?;
        let page = sub_issue_page(&response, number).map_err(GhError::Failed)?;
        sub_issues.extend(listed_sub_issues(page, &response, &nodes));
        cursor = next_page(page);
        if cursor
            .as_ref()
            .is_some_and(|next| !seen.insert(next.clone()))
        {
            return Ok((sub_issues, true));
        }
    }
    Ok((sub_issues, false))
}

// A page with no sub-issues to read fails, since its sub-issues can't be counted. An error
// on one sub-issue leaves only that one unreadable.
fn sub_issue_page(response: &Value, number: u64) -> Result<&Value, String> {
    let pointer = format!("/data/repository/i{number}/subIssues");
    match response.pointer(&pointer) {
        Some(page) if page.is_object() => Ok(page),
        _ => Err(first_error(response)),
    }
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
    let failed = || match stderr.is_empty() {
        true => GhError::Failed(format!("`gh` exited with {}", output.status)),
        false => GhError::Failed(stderr.clone()),
    };
    let response: Value = serde_json::from_slice(&output.stdout).map_err(|_| failed())?;
    // A failure without GraphQL errors, such as an HTTP 403 rate limit or a 502, has its
    // reason only in `gh`'s message.
    if !output.status.success() && response.get("errors").is_none() {
        return Err(failed());
    }
    Ok(response)
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

    #[test]
    fn reads_an_issue_with_its_type_parent_sub_issues_and_next_page() {
        let response = r#"{"data":{"repository":{"i41":{"number":41,"title":"Search",
            "state":"CLOSED","body":"spec-ref: 4f1c2ab","createdAt":"2026-10-01T09:30:00Z",
            "issueType":{"name":"Epic"},
            "parent":{"number":1,"repository":{"nameWithOwner":"acme/roadmap"}},
            "subIssues":{"pageInfo":{"hasNextPage":true,"endCursor":"c1"},
              "nodes":[{"number":3,"repository":{"nameWithOwner":"acme/web"}}]}},
            "i99":null,"i7":null}},
            "errors":[{"type":"NOT_FOUND","path":["repository","i99"],"message":"gone"},
                      {"type":"FORBIDDEN","path":["repository","i7"],"message":"no access"}]}"#;
        let response: Value = serde_json::from_str(response).unwrap();
        let repo = RepoRef::parse("acme/docs").unwrap();

        let read = read_aliases(&response, &[41, 99, 7], |value, alias| {
            read_issue(value, alias, &repo, &response)
        })
        .unwrap();

        let [
            (41, Answer::Found(Read(issue, Some(cursor)))),
            (99, Answer::NotFound),
            (7, Answer::Failed(message)),
        ] = read.as_slice()
        else {
            panic!("expected #41 found with a next page, #99 not found and #7 failed");
        };
        assert_eq!(cursor, "c1");
        assert_eq!(message, "no access");
        let issue_ref = |text: &str| IssueRef::parse(text).unwrap();
        assert_eq!(
            *issue,
            Issue {
                reference: issue_ref("acme/docs#41"),
                title: "Search".into(),
                state: IssueState::Closed,
                issue_type: Some("Epic".into()),
                parent: Some(issue_ref("acme/roadmap#1")),
                sub_issues: vec![SubIssue::Issue(issue_ref("acme/web#3"))],
                text: Some(IssueText {
                    body: "spec-ref: 4f1c2ab".into(),
                    created_at: "2026-10-01T09:30:00Z".into(),
                }),
            }
        );
    }

    #[test]
    fn fails_a_sub_issue_page_with_errors() {
        let response: Value =
            serde_json::from_str(r#"{"data":null,"errors":[{"message":"bad cursor"}]}"#).unwrap();
        assert_eq!(sub_issue_page(&response, 41), Err("bad cursor".into()));
    }
}
