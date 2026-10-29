---
description: Find open issues whose Manifest Docs node changed after they were planned, and help act on them
allowed-tools: Bash(manifestdocs stale:*), Bash(manifestdocs show:*), Bash(manifestdocs context:*), Bash(gh issue view:*), Bash(git diff:*), Bash(git log:*)
---

Run `manifestdocs stale --json`. It lists each open issue linked to a node whose files changed after the issue's `spec-ref`, or after the docs commit from when the issue was created, with the files that changed. It reads the docs clone as of its last `git fetch`. Exit 0 means nothing is stale, 1 means it reported stale issues or issues it couldn't check, and 2 means it couldn't run, with the reason in the message.

For each stale issue:

1. Read what changed in the docs clone with `git diff <since> <changed_in> -- <changed files>`, or `git log -p <changed_in> -- <changed files>` when `since` is `null`, and read the issue with `gh issue view <number> --repo <owner/name>`.
2. Say whether the change affects the issue's work, such as a requirement or scenario it covers that was added, changed or removed, or whether it's unrelated.
3. Suggest one next step: a comment on the issue that says what changed and links to the node, a new `spec-ref:` line set to `changed_in` once the issue matches the node again, or nothing.

Draft each comment or body edit and show it to the user. Run `gh issue comment` or `gh issue edit` only for the ones they approve, each as a visible command. For each issue that wasn't checked, give the reason `stale` printed; a `spec-ref` that isn't a commit in the docs clone may need a `git fetch` there.
