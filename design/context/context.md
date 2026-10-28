---
id: fd-54bf4e
title: "Agent context"
status: draft
summary: "Assembles a node's or an issue's context for a coding agent within an approximate token budget."
---

# Agent context

`mfd context --format md --budget <tokens>` writes context for agents. The budget is approximate. The tool estimates tokens as characters divided by three and reserves 10% headroom. That overestimates for tokenizers near four characters per token, and allows for newer tokenizers, such as recent Claude models', that produce more tokens for the same text. It fills the budget in this order: the target node in full, the root node in full when the tree has one, linked docs, the issue, parent summaries, ancestor summaries, then child summaries. The root node goes in full because it states the rules that apply across the project, such as a game's core rules or a service's delivery guarantee. The tool trims from the end and lists what it left out by path, so an agent can read it. Without `--budget`, nothing is trimmed. Either way, the output ends with its estimated size, and the plugin's hooks and skills always pass a budget.

## Linked docs

`context` follows links one hop from the target node's body. Local Markdown files load in order of first appearance, without duplicates, skipping the target and root nodes. Links inside code spans and fenced blocks don't count. Links to other nodes appear as one-line summaries, and other links, such as images and external URLs, are listed by path. Links inside loaded docs are listed, not followed, so there are no cycles.

## Example

Agent format for an issue:

```markdown
# Context: example-org/cookbook-docs#112 (docs origin/main @ 3f9a1c2, 2026-10-27)

## Issue
Let readers filter search results by cooking time. Acceptance criteria: ...

## Parent work
- #41 Recipe search (open)
  - #58 Search filters (open, 3 of 5 done)

## Feature: Recipe search (fd-7c2e41, approved)
Path: recipes/search
<full node body>

## Project: Cookbook (fd-3e9d0a, approved)
<full root node body>

## Decisions
- dc-3f1a8c2 (accepted): Search results show 20 recipes per page. Readers rarely scroll past 20 before refining a search.
- dc-8b07e15 (proposed): Filters apply as the reader picks them, not on submit.

## Linked docs
### adr/2026-09-search-engine.md
<full doc>

## Around this feature
- Parent: Recipes: <summary>
- Child: Ranking (fd-91ab5d, draft): <summary>

Omitted for budget: recipes/search/ranking-rules.md (2,400 tokens)
Estimated size: 7,100 of 8,000 tokens
```
