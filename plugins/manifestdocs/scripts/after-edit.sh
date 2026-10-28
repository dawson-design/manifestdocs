#!/bin/sh
# After Claude edits a Markdown file or a manifestdocs.yaml in a Manifest Docs tree, checks
# that tree. When it has errors, or can't be checked, such as after a bad config edit, exit 2
# shows the report to Claude, so the tree is fixed in the same turn. Edits to other files,
# and to files outside a tree, pass silently.

command -v manifestdocs >/dev/null 2>&1 || exit 0
# The hook input is one line of JSON. Inside a string value a quote is `\"`, so only the
# real `file_path` key matches, and its value is unescaped.
file=$(sed -n -E 's/.*"file_path" *: *"(([^"\\]|\\.)*)".*/\1/p' | head -n 1 | sed -E 's/\\(.)/\1/g')
case "$file" in
  *.md | */manifestdocs.yaml) ;;
  *) exit 0 ;;
esac
# From the project folder, so the report's paths read from where Claude works.
cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || exit 0

report=$(manifestdocs check "$file" 2>&1)
case $? in
  0) exit 0 ;;
  1) ;;
  *)
    # A file outside every tree is no concern of this hook.
    case "$report" in
      *"isn't in a tree"* | *"no manifestdocs.yaml in this folder"*) exit 0 ;;
    esac
    ;;
esac
printf '%s\n' "manifestdocs check found problems after editing $file:" "$report" >&2
exit 2
