---
id: dc-ebfc707
node: fd-622eed
title: "One setting decides how the skills publish docs changes"
status: accepted
topic: changes-setting
decided_by: alastair
date: 2026-10-29
---

**Decision.** A tree's `changes` setting, `review`, `proposed` or `direct`, decides how the skills publish spec changes and decision records. `review` and `proposed` open a pull request, and `direct` pushes to the docs repo's default branch with no pull request. It replaces `decisions.merge`. The docs repo's branch rules enforce it, and a rejected push falls back to a pull request.

**Why.** Teams run their docs differently: some want every docs change reviewed, and others are happy for agents to update docs straight away. A push is a visible `git` call, as a pull request is a visible `gh` call, so the binary still never writes to GitHub.

**Rejected.** A GitHub App or bot role, which the setting doesn't need, because agents push and open pull requests with the engineer's own `git` and `gh` auth. Separate settings for spec changes and decisions, until a team asks for them.
