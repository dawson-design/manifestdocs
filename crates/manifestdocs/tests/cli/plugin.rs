//! The Claude Code plugin's hook scripts, run as Claude Code runs them: with the hook's JSON
//! on stdin, the project folder in `CLAUDE_PROJECT_DIR`, and `manifestdocs` on the PATH.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::support::{Fixture, Run, blocked_gh, node};

fn script(name: &str) -> PathBuf {
    let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/manifestdocs/scripts");
    scripts.join(name)
}

/// Runs a hook script from `/`, for the project in `project`. Without `binary`,
/// `manifestdocs` isn't on the PATH. A `gh` that always fails comes first, since a runner
/// can have `/usr/bin/gh`.
fn hook(name: &str, input: &str, project: &Path, binary: bool) -> Run {
    let binary_folder = Path::new(env!("CARGO_BIN_EXE_manifestdocs"))
        .parent()
        .unwrap();
    let no_gh = blocked_gh().display();
    let path = match binary {
        true => format!("{no_gh}:{}:/usr/bin:/bin", binary_folder.display()),
        false => format!("{no_gh}:/usr/bin:/bin"),
    };
    let mut child = Command::new("/bin/sh")
        .arg(script(name))
        .current_dir("/")
        .env("PATH", path)
        .env("HOME", project)
        .env("CLAUDE_PROJECT_DIR", project)
        .env("GIT_CEILING_DIRECTORIES", project.parent().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the script runs");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    Run {
        code: output.status.code().expect("the script exits with a code"),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// The JSON Claude Code sends after an edit. `old_string` holds a decoy `file_path`, escaped
/// the way JSON escapes it.
fn edit_input(file: &Path) -> String {
    let file = file
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(
        r#"{{"hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{{"file_path":"{file}","old_string":"see \"file_path\": \"decoy.md\"","new_string":"x"}},"tool_response":{{"filePath":"{file}"}}}}"#
    )
}

/// A code repo with its tree in `docs/` and a pointer to it, whose root node's title is
/// `root_title` as written.
fn repo(name: &str, root_title: &str) -> Fixture {
    let root = format!(
        "---\nid: fd-000001\ntitle: {root_title}\nstatus: draft\nsummary: \"Cookbook summary.\"\n---\n"
    );
    Fixture::new(
        name,
        &[
            (".manifestdocs/config.yaml", "tree: docs\n"),
            ("README.md", "# Cookbook\n"),
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &root),
            ("docs/search/search.md", &node("fd-000002", "Search")),
            ("docs/search/notes.txt", "notes\n"),
            ("docs/say \"hi\"/hi.md", "# Hi\n"),
        ],
    )
}

#[test]
fn after_an_edit_shows_claude_the_errors_in_the_tree() {
    let fixture = repo("plugin-edit-errors", "Cookbook");
    for file in ["docs/search/search.md", "docs/say \"hi\"/hi.md"] {
        let file = fixture.path(file);

        let run = hook("after-edit.sh", &edit_input(&file), &fixture.path(""), true);

        assert_eq!(
            run.stderr,
            format!(
                "manifestdocs check found problems after editing {}:\n\
                 docs/cookbook.md:3: error: `title` must be double-quoted\n\n2 nodes: 1 error\n",
                file.display()
            )
        );
        assert_eq!(run.code, 2);
    }
}

#[test]
fn after_an_edit_stays_quiet_unless_the_edited_files_tree_has_errors() {
    let broken = repo("plugin-edit-quiet-broken", "Cookbook");
    let clean = repo("plugin-edit-quiet-clean", "\"Cookbook\"");
    let uncloned = Fixture::new(
        "plugin-edit-quiet-uncloned",
        &[
            (".manifestdocs/config.yaml", "tree: ../cookbook-docs\n"),
            ("README.md", "# Cookbook\n"),
        ],
    );
    let cases = [
        ("clean tree", &clean, "docs/search/search.md", true),
        ("not Markdown", &broken, "docs/search/notes.txt", true),
        ("outside the tree", &broken, "README.md", true),
        (
            "pointer to a docs repo not cloned",
            &uncloned,
            "README.md",
            true,
        ),
        ("no binary", &broken, "docs/search/search.md", false),
    ];
    for (name, fixture, file, binary) in cases {
        let input = edit_input(&fixture.path(file));

        let run = hook("after-edit.sh", &input, &fixture.path(""), binary);

        let output = (run.stdout.as_str(), run.stderr.as_str());
        assert_eq!(output, ("", ""), "{name}");
        assert_eq!(run.code, 0, "{name}");
    }
}

#[test]
fn at_session_start_adds_an_outline_of_the_tree() {
    let fixture = repo("plugin-session", "\"Cookbook\"");
    let outside = Fixture::new("plugin-session-outside", &[("notes.md", "# Notes\n")]);

    let inside = hook("session-start.sh", "{}", &fixture.path(""), true);
    let elsewhere = hook("session-start.sh", "{}", &outside.path(""), true);

    assert_eq!(
        inside.stdout,
        "This project's requirements and decisions are a Manifest Docs tree. Before planning work on a feature, load its context with the manifestdocs-context skill.\n\
         Cookbook (fd-000001, draft)  docs/cookbook.md\n\
         \x20 Search (fd-000002, draft)  docs/search/search.md\n"
    );
    assert_eq!(inside.code, 0);
    assert_eq!((elsewhere.stdout.as_str(), elsewhere.code), ("", 0));
}

/// `hooks.json` runs the scripts directly, so they must be executable.
#[test]
fn hook_scripts_are_executable() {
    for name in ["after-edit.sh", "session-start.sh"] {
        let mode = fs::metadata(script(name)).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "{name}");
    }
}

#[test]
fn after_an_edit_reports_a_config_that_stops_the_check() {
    let fixture = repo("plugin-edit-config", "\"Cookbook\"");
    let config = fixture.path("docs/manifestdocs.yaml");
    fs::write(&config, "name: cookbook\nnode_tokens: 0\n").unwrap();

    let after_node = hook(
        "after-edit.sh",
        &edit_input(&fixture.path("docs/search/search.md")),
        &fixture.path(""),
        true,
    );
    let after_config = hook(
        "after-edit.sh",
        &edit_input(&config),
        &fixture.path(""),
        true,
    );

    for (run, file) in [
        (&after_node, "docs/search/search.md"),
        (&after_config, "docs/manifestdocs.yaml"),
    ] {
        assert_eq!(
            run.stderr,
            format!(
                "manifestdocs check found problems after editing {}:\n\
                 error: docs/manifestdocs.yaml:2: `node_tokens` must be at least 1\n",
                fixture.path(file).display()
            )
        );
        assert_eq!(run.code, 2);
    }
}
