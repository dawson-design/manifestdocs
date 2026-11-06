---
id: dc-5222da8
node: fd-f44774
title: "Release from the public source repo with Homebrew and a shell installer"
status: accepted
topic: release-channels
decided_by: "alastair"
date: 2026-11-06
supersedes: dc-6f221d1
---

**Decision.** The source repo, `dawson-design/manifestdocs`, is public, and each release is published there. People install with Homebrew from `dawson-design/homebrew-tap` or with dist's shell installer, and both put `manifestdocs` and `mfd` on PATH. There's no `gh` extension. The plugin installs from the marketplace in the source repo, and tests still run against mock projects.

**Why.** A public source repo needs no separate repo for releases. Every skill and hook looks for `manifestdocs` on PATH, which an install as a `gh` extension doesn't provide.

**Rejected.** A private source repo, with releases in a public `gh-manifestdocs` repo that also served `gh extension install`, because a public source repo doesn't need it and the plugin can't find a binary installed that way.
