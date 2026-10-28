use std::process::Command;

#[test]
fn prints_its_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_manifestdocs"))
        .arg("--version")
        .output()
        .expect("the binary runs");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(
        stdout.trim(),
        format!("manifestdocs {}", env!("CARGO_PKG_VERSION"))
    );
}
