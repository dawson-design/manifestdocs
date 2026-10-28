---
id: dc-7204120
node: fd-54bf4e
title: "Estimate tokens as characters divided by three"
status: accepted
topic: token-estimate
decided_by: alastair
date: 2026-10-28
---

**Decision.** The tool estimates tokens as characters divided by three, with 10% headroom.

**Why.** Characters divided by four matches OpenAI's and Google's guidance, but Anthropic's [token-counting docs](https://platform.claude.com/docs/en/build-with-claude/token-counting) say Claude Opus 4.7 and later produce about 30% more tokens for the same text. Dividing by three overestimates for the others and comes close for Claude.

**Rejected.** Characters divided by four. An exact tokenizer, which can come later behind a flag.
