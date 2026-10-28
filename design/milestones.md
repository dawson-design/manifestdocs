# Manifest Docs: milestones

Manifest Docs dogfoods itself as early as each milestone allows. Its design docs are its first tree, and the decisions from its PRD are its first decision records, written by hand until M2 builds `decide`.

| Milestone | Scope | Done when |
|---|---|---|
| M0. Format | `manifestdocs.yaml`, node schema and IDs, ownership, the `.manifestdocs` pointer, `init`, `tree`, `show`, `check` | The Manifest Docs design tree and the Root and Branch docs repo pass `check` |
| M1. Offline context | `context <id>`, linked docs, budget trimming, the node size warning in `check`, plugin with `manifestdocs-context`, `manifestdocs-author` and `manifestdocs-verify`, and the explore, propose, apply and verify commands | Sessions building Manifest Docs, and personal factory agents, start from `mfd context` |
| M2. Shared decisions | Decision record format, `decide`, `decisions`, `show <dc-id>`, decisions in `context`, reading the default branch from other repos, `manifestdocs-decide` skill and the record command, end-of-session hook, sandbox write path | A decision made in one Manifest Docs build session appears in another agent's next session once its PR merges |
| M3. Work tree | `context <issue>`, `work`, `stale`, `spec-ref` with the creation-time fallback, `manifestdocs-plan-issue` and `manifestdocs-capture` skills, the remaining commands | `agent-run` builds `.context/` from an issue in the docs repo, and one feature goes from node to linked issues to a merged decision record |
| M4. 1.0 | `dist` releases, Homebrew tap, `gh` extension, plugin marketplace repo | Installs on a fresh Mac and in a Linux factory container without a local build |
| Later | `mfd mcp` | Demand from a client without a shell |
