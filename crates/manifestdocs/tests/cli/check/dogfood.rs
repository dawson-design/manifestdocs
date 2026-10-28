use std::path::Path;

use crate::support::run_in;

/// The project's own design docs are its first tree, so they pass its own check.
#[test]
fn passes_the_design_tree() {
    let design = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design");
    let home = Path::new(env!("CARGO_TARGET_TMPDIR"));

    let run = run_in(&design, home, &["check"]);

    assert!(
        run.stdout.ends_with(" nodes: no problems\n") && run.stdout.lines().count() == 1,
        "{}",
        run.stdout
    );
    assert_eq!(run.code, 0);
}
