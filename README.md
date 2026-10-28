# Manifest Docs

A CLI that adds shared, living context on top of GitHub. The design tree starts at [design/manifest-docs.md](design/manifest-docs.md).

## Build and test

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## Install

```sh
cargo install --path crates/manifestdocs
```

## Check a tree

Run `manifestdocs check` anywhere inside a tree. It exits 1 when the tree has errors and 2 when it can't run.
