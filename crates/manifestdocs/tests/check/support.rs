use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// A throwaway repo in the temp folder, removed when dropped.
pub struct Fixture {
    root: PathBuf,
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Fixture {
    /// Writes `files` under a new folder with an empty `.git` folder, so the folder reads as a repo root.
    pub fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("mfd-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".git")).expect("create the fixture");
        for (path, text) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().expect("a file has a folder"))
                .expect("create a folder");
            fs::write(&path, text).expect("write a fixture file");
        }
        Self { root }
    }

    /// Runs `manifestdocs check` from `folder`, a path relative to the fixture root.
    pub fn check(&self, folder: &str) -> Run {
        let output = Command::new(env!("CARGO_BIN_EXE_manifestdocs"))
            .arg("check")
            .current_dir(self.root.join(folder))
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join(".config"))
            .output()
            .expect("the binary runs");
        Run {
            code: output.status.code().expect("the binary exits with a code"),
            stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
            stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A node file with valid frontmatter.
pub fn node(id: &str, title: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\nstatus: draft\nsummary: \"{title} summary.\"\n---\n\n# {title}\n"
    )
}
