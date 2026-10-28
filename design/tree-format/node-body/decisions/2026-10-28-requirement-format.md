---
id: dc-d048f3d
node: fd-4b7c51
title: "Requirements are a writing convention, not a parsed format"
status: accepted
topic: requirement-format
decided_by: alastair
date: 2026-10-28
---

**Decision.** Nodes can write requirements as `### Requirement:` headings with SHALL or MUST, each with `#### Scenario:` headings and WHEN and THEN lines. The author and verify skills use the convention. The binary doesn't parse it, and `check` doesn't enforce it. Implementation detail goes in a linked supporting doc or a child node, not the body.

**Why.** Scenarios give agents acceptance criteria they can test, and give the verify step something to check a diff against. Plain headings read well on GitHub, and their anchors let issues and records link to one requirement, which `check` already validates. Keeping detail out of the body keeps the ancestors `context` loads in full small.

**Rejected.** `show --requirements` and a `check` warning for a requirement with no scenario, which add format rules to the core before the convention has proved itself. Separate files for product and technical specs, since one tree holds both.
