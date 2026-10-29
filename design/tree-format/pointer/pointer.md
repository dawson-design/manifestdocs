---
id: fd-5b2bd2
title: "Pointer"
status: draft
summary: "How a code repo or workspace folder names its tree with a `.manifestdocs/config.yaml` pointer file."
---

# Pointer

Walking up never reaches the root when the tree is in a subfolder of the code repo, such as `docs/`, or in a docs repo cloned beside the code repos. A pointer file, `.manifestdocs/config.yaml`, names the tree for every folder under the folder it's in. `tree` is relative to that folder.

```yaml
# cookbook/.manifestdocs/config.yaml: a single repo, committed
tree: docs
```

```yaml
# ~/work/cookbook/.manifestdocs/config.yaml: a workspace folder holding the code repos and the docs repo
tree: cookbook-docs
```

The nearest pointer at or above the current folder wins, so a repo's own pointer comes before one in a folder above it. A `.manifestdocs/` folder without `config.yaml`, such as one holding only decision drafts, isn't a pointer. Whether the tree is in the working repo follows from where the pointer leads, and isn't a setting.
