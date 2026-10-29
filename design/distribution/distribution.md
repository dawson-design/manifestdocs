---
id: fd-f44774
title: "Distribution"
status: draft
summary: "Where the code is stored and how people install the tool before and after 1.0."
---

# Distribution

- The source repo and test repos are private, in the `dawson-design` GitHub org. Tests run against mock projects.
- Before 1.0, users build and install from a local clone with `cargo install --path`, and the Claude Code plugin installs from a local marketplace folder.
- At 1.0, `dist` (cargo-dist) builds releases for macOS ARM, Linux x86 and Linux ARM, with a formula in the `dawson-design/homebrew-tap` tap. Because the source repo is private, each release is published to the public `gh-manifestdocs` repo, which also carries the binary under gh's naming convention, so `gh extension install` works and the tool runs as `gh manifestdocs`. The plugin installs from a marketplace repo.
- Releasing needs `gh-manifestdocs` created public with one commit, since dist tags its default branch's latest commit. The source repo needs two Actions secrets: `GH_RELEASES_TOKEN`, which can write releases on `gh-manifestdocs`, and `HOMEBREW_TAP_TOKEN`, which can push to the tap.
