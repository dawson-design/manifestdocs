//! `mfd`, the short name for `manifestdocs`.

use std::path::Path;
use std::process::Command;

use crate::support::{Fixture, Run, node, path_without_gh, run_in};

/// Runs `mfd` with `args` in `folder`, with the same environment as `support::run_in`.
fn run_mfd(folder: &Path, home: &Path, args: &[&str]) -> Run {
    let ceiling = home.parent().unwrap_or(home);
    let output = Command::new(env!("CARGO_BIN_EXE_mfd"))
        .args(args)
        .current_dir(folder)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .env("PATH", path_without_gh())
        .output()
        .expect("the binary runs");
    Run {
        code: output.status.code().expect("the binary exits with a code"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

#[test]
fn mfd_prints_what_manifestdocs_prints() {
    let root = node("fd-000001", "Cookbook");
    let search = node("fd-000002", "Search");
    let fixture = Fixture::new(
        "mfd-alias",
        &[
            ("manifestdocs.yaml", "name: cookbook\n"),
            ("cookbook.md", root.as_str()),
            ("search/search.md", search.as_str()),
            ("search/notes.md", "[missing](gone.md)\n"),
        ],
    );
    let folder = fixture.path("");

    for args in [&["tree"][..], &["show", "fd-000002"], &["check"]] {
        let long = run_in(&folder, &folder, args);
        let short = run_mfd(&folder, &folder, args);

        assert_eq!(short.stdout, long.stdout, "stdout of {args:?}");
        assert_eq!(short.stderr, long.stderr, "stderr of {args:?}");
        assert_eq!(short.code, long.code, "exit code of {args:?}");
    }
}
