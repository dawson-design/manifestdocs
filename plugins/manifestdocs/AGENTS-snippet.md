## Manifest Docs

This project's requirements and decisions are a Manifest Docs tree. Copy this section into `AGENTS.md` for agent tools that don't load skills.

- Before planning work on a feature, run `manifestdocs context <node id or path> --format md --budget 8000` and read the whole output. `manifestdocs tree --depth 2` lists the nodes and their IDs.
- A node is a folder with a Markdown file of the same name, such as `recipes/search/search.md`. Its frontmatter has `id` (`fd-` and 6 hex characters, never changed), a double-quoted `title` and `summary`, and a `status` from `manifestdocs.yaml`.
- After editing anything in the tree, run `manifestdocs check` and fix every error it reports in the files you touched.
