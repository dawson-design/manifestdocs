# make_workdir <arm> <dest>: copies the fixture to <dest> as a fresh git repo set up for
# one arm, and tags the starting commit `eval-base` so check.sh can diff against it.
#
# AGENTS.md is the same in every arm. Arms B and C also get a .manifestdocs pointer to docs/,
# which a project using Manifest Docs commits; their image adds the binaries and the plugin.

EVALS=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
REPO_ROOT=$(cd "$EVALS/.." && pwd)

make_workdir() {
  local arm=$1 dest=$2
  mkdir -p "$dest"
  rsync -a --exclude __pycache__ "$EVALS/fixture/" "$dest/"
  case "$arm" in
    A) ;;
    B | C)
      mkdir -p "$dest/.manifestdocs"
      printf 'tree: docs\n' >"$dest/.manifestdocs/config.yaml"
      ;;
    *)
      echo "unknown arm: $arm" >&2
      return 2
      ;;
  esac
  (
    cd "$dest" &&
      git init -q &&
      git add -A &&
      git -c user.name=eval -c user.email=eval@example.invalid commit -q -m "Fixture" &&
      git tag eval-base
  )
}
