#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
checker="$script_dir/check-migration-immutability.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

git -C "$fixture" init --quiet --initial-branch=main
git -C "$fixture" config user.email "migration-test@example.com"
git -C "$fixture" config user.name "Migration Test"
mkdir -p "$fixture/database/migrations"
printf 'CREATE TABLE users (id BIGINT PRIMARY KEY);\n' > "$fixture/database/migrations/001_users.sql"
printf 'ALTER TABLE users ADD COLUMN name TEXT;\n' > "$fixture/database/migrations/002_user_names.sql"
git -C "$fixture" add database/migrations
git -C "$fixture" commit --quiet -m "Add sample migrations"
base_commit=$(git -C "$fixture" rev-parse HEAD)

assert_check() {
  local expected_status=$1
  local expected_output=$2
  local output
  local actual_status

  if output=$(cd "$fixture" && bash "$checker" "$base_commit" 2>&1); then
    actual_status=0
  else
    actual_status=$?
  fi

  if [[ $actual_status -ne $expected_status || "$output" != *"$expected_output"* ]]; then
    echo "ERROR: expected status $expected_status and output containing: $expected_output" >&2
    echo "$output" >&2
    exit 1
  fi

  if [[ $actual_status -ne 0 ]]; then
    echo "$output"
  fi
}

git -C "$fixture" switch --quiet -c unchanged
git -C "$fixture" commit --quiet --allow-empty -m "No migration changes"
assert_check 0 "Migration checksums match the base branch"

git -C "$fixture" switch --quiet -C added "$base_commit"
printf 'CREATE INDEX users_name_idx ON users (name);\n' > "$fixture/database/migrations/003_user_name_index.sql"
git -C "$fixture" add database/migrations/003_user_name_index.sql
git -C "$fixture" commit --quiet -m "Add migration"
assert_check 0 "Migration checksums match the base branch"

git -C "$fixture" switch --quiet -C modified "$base_commit"
printf 'CREATE TABLE users (id UUID PRIMARY KEY);\n' > "$fixture/database/migrations/001_users.sql"
git -C "$fixture" add database/migrations/001_users.sql
git -C "$fixture" commit --quiet -m "Modify migration"
assert_check 1 "ERROR: modified merged migration: database/migrations/001_users.sql"

git -C "$fixture" switch --quiet -C deleted "$base_commit"
git -C "$fixture" rm --quiet database/migrations/002_user_names.sql
git -C "$fixture" commit --quiet -m "Delete migration"
assert_check 1 "ERROR: deleted merged migration: database/migrations/002_user_names.sql"

git -C "$fixture" switch --quiet -C typechange "$base_commit"
git -C "$fixture" rm --quiet database/migrations/001_users.sql
ln -s 002_user_names.sql "$fixture/database/migrations/001_users.sql"
git -C "$fixture" add database/migrations/001_users.sql
git -C "$fixture" commit --quiet -m "Convert migration to symlink"
assert_check 1 "ERROR: modified merged migration: database/migrations/001_users.sql"

echo "Migration immutability tests passed"
