---
id: fd-f44774
title: "Distribution"
status: draft
summary: "Where the code is stored and how people install the tool before and after 1.0."
---

# Distribution

- The source repo, `dawson-design/manifestdocs`, is public. Tests run against mock projects.
- Before 1.0, users build and install from a clone with `cargo install --path crates/manifestdocs`.
- At 1.0, `dist` (cargo-dist) builds releases for macOS ARM, Linux x86 and Linux ARM, and publishes each one on the source repo. People install with Homebrew, from a formula in the `dawson-design/homebrew-tap` tap, or with dist's shell installer, such as in a Linux container. Both install `manifestdocs` and its short name `mfd`. There's no `gh` extension, because the plugin's skills and hooks look for `manifestdocs` on PATH.
- The plugin installs from the marketplace in the source repo, with `claude plugin marketplace add dawson-design/manifestdocs`.
- Releasing needs one Actions secret on the source repo, `HOMEBREW_TAP_TOKEN`, which can push to the tap.
