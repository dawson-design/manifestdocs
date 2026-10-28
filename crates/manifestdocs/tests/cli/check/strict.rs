use crate::support::{Fixture, node};

const CONFIG: (&str, &str) = ("manifestdocs.yaml", "name: demo\n");

#[test]
fn strict_fails_a_tree_with_only_warnings() {
    let recipes = node("fd-000002", "Recipes");
    let fixture = Fixture::new(
        "strict-warning",
        &[CONFIG, ("recipes/recipes.md", &recipes)],
    );

    let run = fixture.run("", &["check", "--strict"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: no root node `demo.md`\n\n1 node: 1 warning\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn strict_passes_a_tree_with_no_problems() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new("strict-clean", &[CONFIG, ("demo.md", &root)]);

    let run = fixture.run("", &["check", "--strict"]);

    assert_eq!(run.stdout, "1 node: no problems\n");
    assert_eq!(run.code, 0);
}
