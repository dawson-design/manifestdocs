#!/bin/sh
# Adds a one-level outline of the Manifest Docs tree to the session's context, when the
# session's folder is in a tree or a pointer leads to one. Otherwise it prints nothing.

command -v manifestdocs >/dev/null 2>&1 || exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || exit 0
outline=$(manifestdocs tree --depth 1 2>/dev/null) || exit 0
[ -n "$outline" ] || exit 0

printf '%s\n' "This project's requirements and decisions are a Manifest Docs tree. Before planning work on a feature, load its context with the manifestdocs-context skill."
printf '%s\n' "$outline"
