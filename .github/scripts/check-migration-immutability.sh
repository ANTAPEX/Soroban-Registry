#!/usr/bin/env bash

set -euo pipefail

base_ref=${1:-origin/main}
migrations_dir="database/migrations"

if ! git rev-parse --verify --quiet "${base_ref}^{commit}" >/dev/null; then
  echo "ERROR: base ref not found: $base_ref" >&2
  exit 1
fi

merge_base=$(git merge-base "$base_ref" HEAD)
status=0

while IFS= read -r -d '' change && IFS= read -r -d '' file_path; do
  case "$change" in
    M)
      base_checksum=$(git show "$merge_base:$file_path" | sha256sum | cut -d ' ' -f 1)
      head_checksum=$(git show "HEAD:$file_path" | sha256sum | cut -d ' ' -f 1)
      echo "ERROR: modified merged migration: $file_path" >&2
      echo "  base SHA-256: $base_checksum" >&2
      echo "  head SHA-256: $head_checksum" >&2
      status=1
      ;;
    D)
      echo "ERROR: deleted merged migration: $file_path" >&2
      status=1
      ;;
  esac
done < <(git diff --no-renames --name-status -z --diff-filter=MD "$base_ref"...HEAD -- "$migrations_dir")

if [[ $status -ne 0 ]]; then
  echo "Merged migrations are immutable. Add a new migration to fix the schema forward." >&2
  exit $status
fi

echo "Migration checksums match the base branch"
