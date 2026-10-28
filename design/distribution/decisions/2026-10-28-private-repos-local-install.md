---
id: dc-6f221d1
node: fd-f44774
title: "Private repos, mock test projects, local installs until 1.0"
status: accepted
topic: private-repos-local-install
decided_by: alastair
date: 2026-10-28
---

**Decision.** The source and test repos are private in the `dawson-design` GitHub org, and tests run against mock projects. Before 1.0, people install from a local build. Homebrew and the `gh` extension wait for 1.0.

**Why.** The tool is pre-release, and test projects must not copy any client's code or docs.

**Rejected.** Publishing through the public `homebrew-tap` and `claude-plugins` repos before 1.0.
