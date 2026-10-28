---
id: dc-5ee09fb
node: fd-e54a4d
title: "A node owns every file under its folder"
status: accepted
topic: file-ownership
decided_by: alastair
date: 2026-10-28
---

**Decision.** A node owns every file under its folder, except files under a deeper node or a nested tree. `show` lists them and `stale` watches them. `context` loads only the ones the node links to.

**Why.** Bazel assigns files to the nearest package above them in the same way. No tool researched limits ownership to the files directly inside a folder.

**Rejected.** Ownership of only the files directly inside a node folder, which leaves files in subfolders such as `images/` with no owner.
