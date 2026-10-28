---
id: dc-518c345
node: fd-e54a4d
title: "A node file is named after its folder"
status: accepted
topic: node-file-name
decided_by: alastair
date: 2026-10-28
---

**Decision.** A folder is a node when it contains a Markdown file named after the folder, such as `recipes/recipes.md`, with an `id` in its frontmatter. The root node is named after the tree's `name`.

**Why.** With a `README.md` in every folder, editor tabs and fuzzy-finder results all read the same, which is why Rust moved away from `mod.rs`. The root node can't take its folder's name, because a clone's folder name isn't fixed.

**Rejected.** `README.md`, which GitHub shows when a folder is opened but repeats in every tab. `FEATURE.md`, which repeats in every tab and which GitHub doesn't show. A `recipes.md` file beside the `recipes/` folder, which splits a node across two paths that editors don't list together.
