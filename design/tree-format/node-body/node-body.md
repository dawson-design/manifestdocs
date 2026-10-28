---
id: fd-4b7c51
title: "Node body"
status: draft
summary: "How to write a node's body so agents get behaviour they can build and verify, without detail they don't need."
---

# Node body

A node's body is plain Markdown that reads as an ordinary page on GitHub. The tool reads only its links and headings, and `check` enforces none of what follows. The `manifestdocs-author` skill writes bodies this way, and the `manifestdocs-verify` skill reads them.

## What goes in the body

The body states what the feature must do: its behaviour, its contracts, the constraints that apply to everything below it, and how to accept it. If the implementation could change without changing visible behaviour, the detail belongs elsewhere:

- in a supporting doc in the node's folder, such as `design.md`, linked from the body
- in a child node, when the detail is a feature of its own

Product and technical specs share one tree, so this is about placement, not about leaving technical detail out. It matters because `context` includes the target node and every ancestor in full, but follows links only from the target's body. A design doc linked from a node reaches agents working on that node, not agents working on every node below it.

## Requirements

A node can list its requirements as headings:

```markdown
## Requirements

### Requirement: Filter by cooking time
Search SHALL let readers limit results to recipes they can cook within a chosen time.

#### Scenario: Under 30 minutes
- WHEN a reader picks "under 30 minutes"
- THEN every result has a total time of 30 minutes or less
```

- A requirement states one behaviour, with SHALL or MUST.
- Each requirement has one or more scenarios, with WHEN and THEN lines that a test could check.
- Each heading gets an anchor, such as `search.md#requirement-filter-by-cooking-time`, so issues, decision records and tests can link to one requirement. `check` reports the link when the heading changes.

Bodies in prose stay valid. A node can mix requirements with other sections, such as background or open questions.
