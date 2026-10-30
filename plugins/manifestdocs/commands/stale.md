---
description: Find open issues whose Manifest Docs node changed after they were planned, and help act on them
allowed-tools: Bash(manifestdocs stale:*), Bash(manifestdocs show:*), Bash(manifestdocs context:*), Bash(gh issue view:*)
---

Run `manifestdocs stale --json`. It lists each open issue linked to a node whose files changed after the issue's `spec-ref`, or after the docs commit from when the issue was created, with the files that changed. It reads the docs clone as of its last `git fetch`. Exit 0 means nothing is stale, 1 means it reported stale issues or issues it couldn't check, and 2 means it couldn't run, with the reason in the message.

For each stale issue:

1. Read what changed. The docs clone is the git repo holding the node's `file`, which can be a separate clone reached through a pointer. Run `git -C <docs clone> diff <since.commit> <changed_in.commit> -- <paths>`, or `git -C <docs clone> log -p <changed_in.commit> -- <paths>` when `since` is `null`. `stale` prints each `changes[].path` relative to the current folder, so rewrite it relative to the clone. Read the issue with `gh issue view <number> --repo <owner/name>`.
2. Say whether the change affects the issue's work, such as a requirement or scenario it covers that was added, changed or removed, or whether it's unrelated.
3. Suggest one next step: a comment on the issue that says what changed and links to the node, a new `spec-ref:` line set to `changed_in.commit` once the issue matches the node again, or nothing.

Draft each comment or body edit and show it to the user. Run `gh issue comment` or `gh issue edit` only for the ones they approve, each as a visible command. For each issue that wasn't checked, give the reason `stale` printed; a `spec-ref` that isn't a commit in the docs clone may need a `git fetch` there.
