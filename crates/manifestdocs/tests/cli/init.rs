//! `manifestdocs init`.

use std::fs;

use crate::support::{Fixture, Run};

/// A repo with an empty `cookbook-docs` folder, plus `files`.
fn empty_folder(name: &str, files: &[(&str, &str)]) -> Fixture {
    let fixture = Fixture::new(name, files);
    fs::create_dir_all(fixture.path("cookbook-docs")).expect("create the folder");
    fixture
}

/// The ID in `created manifestdocs.yaml and <file> (<id>)`, checked to be a full node ID.
fn created_id(run: &Run, file: &str) -> String {
    let id = run
        .stdout
        .strip_prefix(&format!("created manifestdocs.yaml and {file} ("))
        .and_then(|rest| rest.strip_suffix(")\n"))
        .unwrap_or_else(|| panic!("unexpected output: {}", run.stdout));
    let hex = id.strip_prefix("fd-").expect("the ID starts with fd-");
    assert!(
        hex.len() == 6 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "{id}"
    );
    id.to_owned()
}

#[test]
fn starts_a_tree_named_after_the_folder_that_passes_check() {
    let fixture = empty_folder("init-folder-name", &[]);

    let run = fixture.run("cookbook-docs", &["init"]);

    let id = created_id(&run, "cookbook-docs.md");
    assert_eq!(run.code, 0);
    let config = fs::read_to_string(fixture.path("cookbook-docs/manifestdocs.yaml")).unwrap();
    let root = fs::read_to_string(fixture.path("cookbook-docs/cookbook-docs.md")).unwrap();
    assert_eq!(config, "name: \"cookbook-docs\"\n");
    assert_eq!(
        root,
        format!(
            "---\nid: {id}\ntitle: \"cookbook-docs\"\nstatus: draft\n\
             summary: \"TODO: one sentence about cookbook-docs.\"\n---\n\n# cookbook-docs\n"
        )
    );
    let check = fixture.check("cookbook-docs");
    assert_eq!(check.stdout, "1 node: no problems\n");
    assert_eq!(check.code, 0);
}

#[test]
fn keeps_a_title_that_yaml_would_misread_intact() {
    let fixture = empty_folder("init-title", &[]);
    let title = "Q&A: \"what's next\" #2 \\ done";

    let run = fixture.run("cookbook-docs", &["init", title]);

    let id = created_id(&run, "q-a-what-s-next-2-done.md");
    let show = fixture.run("cookbook-docs", &["show", "."]);
    assert_eq!(
        show.stdout,
        format!(
            "{title} ({id}, draft)\nFile: q-a-what-s-next-2-done.md\n\
             Summary: TODO: one sentence about {title}.\n\n# {title}\n"
        )
    );
    assert_eq!(fixture.check("cookbook-docs").code, 0);
}

#[test]
fn quotes_a_name_that_yaml_would_read_as_null() {
    let fixture = empty_folder("init-null", &[]);

    let run = fixture.run("cookbook-docs", &["init", "Null"]);

    created_id(&run, "null.md");
    assert_eq!(
        fixture.check("cookbook-docs").stdout,
        "1 node: no problems\n"
    );
}

#[test]
fn refuses_to_replace_a_file_or_make_a_name_from_no_letters() {
    let cases = [
        (
            "init-config-exists",
            "manifestdocs.yaml",
            "Cookbook",
            "error: manifestdocs.yaml already exists\n",
        ),
        (
            "init-root-exists",
            "cookbook.md",
            "Cookbook",
            "error: cookbook.md already exists\n",
        ),
        (
            "init-no-letters",
            "notes.md",
            "!!",
            "error: can't make a tree name from `!!`: give a title with letters or digits\n",
        ),
    ];
    for (name, existing, title, expected) in cases {
        let path = format!("cookbook-docs/{existing}");
        let fixture = empty_folder(name, &[(&path, "keep\n")]);

        let run = fixture.run("cookbook-docs", &["init", title]);

        assert_eq!(run.stderr, expected, "{name}");
        assert_eq!(run.code, 2, "{name}");
        let mut files: Vec<String> = fs::read_dir(fixture.path("cookbook-docs"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, [existing], "{name}");
        let kept = fs::read_to_string(fixture.path(&path)).unwrap();
        assert_eq!(kept, "keep\n", "{name}");
    }
}
