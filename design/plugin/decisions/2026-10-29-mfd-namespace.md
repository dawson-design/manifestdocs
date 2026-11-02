---
id: dc-92fc94d
node: fd-fd80a6
title: "The plugin's commands use the mfd namespace"
status: accepted
topic: plugin-namespace
decided_by: alastair
date: 2026-10-29
---

**Decision.** The plugin is named `mfd`, the binary's alias, so its commands read `/mfd:explore` rather than `/manifestdocs:explore`, and it installs as `mfd@manifestdocs`. The skills keep their `manifestdocs-*` names, and the plugin stays in `plugins/manifestdocs`.

**Why.** A shorter namespace saves space in every command an agent or person types and reads.

**Rejected.** Also shortening the skill names, and moving the plugin's folder to `plugins/mfd`.
