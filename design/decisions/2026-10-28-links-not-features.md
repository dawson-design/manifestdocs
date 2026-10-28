---
id: dc-a82983f
node: fd-622eed
title: "Images, style guides and external references are Markdown links"
status: accepted
topic: links-not-features
decided_by: alastair
date: 2026-10-28
---

**Decision.** Images, diagrams, style guides and external references are ordinary links in node bodies, and Mermaid diagrams go inline. `check` confirms that local links resolve, and the tool doesn't fetch or track external links.

**Why.** Links already work on GitHub, in editors and for agents. Building features for them would repeat the earlier mistake of adding every feature to the tool.

**Rejected.** References that declare their own scope, an asset listing with captions and sizes, and a `refs:` field for external URLs.
