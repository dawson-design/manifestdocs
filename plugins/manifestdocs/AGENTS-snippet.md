## Manifest Docs

This project's requirements and decisions are a Manifest Docs tree. Copy this section into `AGENTS.md` for agent tools that don't load skills.

- Before planning work on a feature, run `manifestdocs context <node id or path> --format md --budget 8000` and read the whole output. `manifestdocs tree --depth 2` lists the nodes and their IDs.
- A node is a folder with a Markdown file of the same name, such as `recipes/search/search.md`. Its frontmatter has `id` (`fd-` and 6 hex characters, never changed), a double-quoted `title` and `summary`, and a `status` from `manifestdocs.yaml`.
- After editing anything in the tree, run `manifestdocs check` and fix every error it reports in the files you touched.
- When the engineer makes a decision worth recording, such as a choice between real alternatives, a departure from the spec or a new convention, draft a record (frontmatter `node`, a double-quoted `title` and `topic`; a body of **Decision.**, **Why.** and **Rejected.**), ask them to confirm it, then pass it to `manifestdocs decide --file -` on a new docs branch and open a PR. Never put secrets or personal data in it.
- When writing an issue for a node, link to the node instead of copying its text, end the body with `spec-ref: <docs commit>` from the `context` header, and add `feature: <node id>` unless the node lists the issue, or a parent of it, in `work`. When an issue thread settles a decision, draft a record for it the same way, with the issue in `source`.
