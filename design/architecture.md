# Manifest Docs: architecture

## Language and crates

Rust, split into a library crate and a thin binary crate.

| Crate | Role |
|---|---|
| `manifestdocs-core` | Tree parsing, ID and link resolution, decision records, context assembly, budget trimming, checks |
| `manifestdocs` | CLI, GitHub reads, output formatting |

| Need | Choice |
|---|---|
| CLI parsing | `clap` |
| Tree walking that respects `.gitignore` | `ignore` |
| Reading the default branch without a checkout | `git symbolic-ref`, `git ls-tree` and one `git cat-file --batch` process, behind the same file-source interface as the working tree; `gix` later |
| Frontmatter and YAML | `serde` with `serde-saphyr`, plus the tool's own key scan for quoting and line numbers |
| Markdown structure | `pulldown-cmark` |
| Git history and SHAs | `git` subprocess first, `gix` later |
| GitHub work tree | Read-only `gh api graphql` subprocess, batched per command |
| IDs | The operating system's random source, through `getrandom` |
| Token estimates | Characters divided by three, with 10% headroom; an exact tokenizer later behind a flag if needed |
| Errors | `thiserror` in core, `anyhow` in the binary |

Shelling out to `gh` reuses the user's auth, SSO and GitHub Enterprise hosts, with no token handling in Manifest Docs.

## Local folder

`.manifestdocs/` is the tool's only local folder. It holds the [tree pointer](tree-format/tree-format.md#pointer), which can be committed, and decision drafts from sandboxed runs, which ignore themselves (see [sandbox use](sandbox/sandbox.md)). There's no cache or index: the tool reads the files on every run.

## Future reuse

The core crate can power a `mfd mcp` subcommand for clients without a shell, or other tools that need to read the tree.
