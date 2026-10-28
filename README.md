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

## Use

- `manifestdocs init [title]` starts a tree in the current folder.
- `manifestdocs tree [path] [--depth n]` prints an outline of the nodes.
- `manifestdocs show <id or path>` prints one node, with its parent and supporting docs.
- `manifestdocs check` validates the tree. It exits 1 when the tree has errors and 2 when it can't run.

`tree`, `show` and `check` work anywhere inside a tree. To use them from a code repo outside the tree, map the repo's folder to the tree in `~/.config/manifestdocs/config.yaml`:

```yaml
workspaces:
  - path: ~/work/cookbook
    tree: ~/work/cookbook/cookbook-docs
```
