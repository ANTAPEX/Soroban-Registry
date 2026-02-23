# Soroban Registry — Automated Changelog Generation

**Version:** 1.0.0 | **Effective:** 2026-02-22 | **Owner:** Release Engineering

---

## Overview

The Changelog Generation feature automates the creation of structured, consistent release notes from git commit history. It parses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/), detects breaking changes, enforces semantic versioning contracts, and makes release history accessible through both the CLI and REST API.

Manual changelogs are inconsistent and error-prone. This feature ensures every release is fully documented, breaking changes are never silently shipped, and consumers of registry contracts can programmatically query what changed between versions.

---

## Architecture

```
┌──────────────┐     git log      ┌─────────────────────┐
│  Git History  │ ───────────────► │  Conventional Commit │
│  (commits)    │                  │  Parser              │
└──────────────┘                  └──────────┬──────────┘
                                             │
                              ┌──────────────┼──────────────┐
                              ▼              ▼              ▼
                     ┌──────────────┐ ┌────────────┐ ┌──────────────┐
                     │  Breaking    │ │  Version   │ │  Markdown    │
                     │  Change     │ │  Bump      │ │  Renderer    │
                     │  Detector   │ │  Enforcer  │ │              │
                     └──────┬───────┘ └─────┬──────┘ └──────┬───────┘
                            │               │               │
                            ▼               ▼               ▼
                     ┌─────────────────────────────────────────────┐
                     │              CHANGELOG.md                    │
                     │         or  Registry API (DB)               │
                     └─────────────────────────────────────────────┘
```

### Components

| Component | Location | Purpose |
|---|---|---|
| CLI Module | `cli/src/changelog.rs` | Local git parsing, markdown generation, version validation |
| API Handlers | `backend/api/src/changelog_handlers.rs` | REST endpoints for storing/querying changelogs |
| Shared Models | `backend/shared/src/models.rs` | Shared types across CLI and API |
| Database Migration | `database/migrations/20260222000000_add_changelog.sql` | Persistent storage schema |

---

## Conventional Commits

The parser recognizes the full [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification:

```
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

### Supported Types

| Type | Section Heading | Description |
|---|---|---|
| `feat` | Features | A new feature |
| `fix` | Bug Fixes | A bug fix |
| `docs` | Documentation | Documentation-only changes |
| `style` | Styles | Formatting, whitespace, semicolons |
| `refactor` | Code Refactoring | Neither fixes a bug nor adds a feature |
| `perf` | Performance Improvements | Performance improvement |
| `test` | Tests | Adding or correcting tests |
| `build` | Build System | Build system or external dependencies |
| `ci` | CI/CD | CI configuration files and scripts |
| `chore` | Chores | Maintenance tasks |
| `revert` | Reverts | Reverting a previous commit |

### Breaking Change Detection

Breaking changes are detected through three mechanisms:

1. **Bang notation:** `feat!: remove deprecated endpoint`
2. **Footer token:** A commit body containing `BREAKING CHANGE: <description>`
3. **Subject prefix:** Description starting with `BREAKING CHANGE`

All breaking changes are surfaced in a dedicated `⚠ BREAKING CHANGES` section at the top of the changelog.

---

## CLI Usage

### Generate a Changelog

```bash
# Basic — generates from latest tag to HEAD, writes CHANGELOG.md
soroban-registry changelog generate --output=CHANGELOG.md

# Specify version explicitly
soroban-registry changelog generate --output=CHANGELOG.md --version=2.0.0

# Custom git range
soroban-registry changelog generate --from=v1.2.0 --to=v2.0.0

# With a release title
soroban-registry changelog generate --version=2.0.0 --title="Aurora Release"

# JSON output (useful for CI pipelines)
soroban-registry changelog generate --json
```

**Flags:**

| Flag | Default | Description |
|---|---|---|
| `--output` | `CHANGELOG.md` | Output file path |
| `--version` | Auto-detected | Target version string |
| `--title` | None | Human-readable release title |
| `--from` | Latest git tag | Start ref (tag or commit hash) |
| `--to` | `HEAD` | End ref |
| `--json` | `false` | Output structured JSON instead of markdown |

### Validate a Version

Checks that a proposed version correctly reflects the changes in the commit range. Fails if:

- Breaking changes exist but major version is not bumped
- New features exist but minor version is not bumped
- The new version is not greater than the current version

```bash
# Validate before releasing
soroban-registry changelog validate 2.0.0

# Validate against a specific base
soroban-registry changelog validate 1.3.0 --from=v1.2.0
```

### Push to Registry

Sends the parsed changelog to the Soroban Registry API for persistent storage and API access.

```bash
soroban-registry changelog push \
  --contract-id=<uuid> \
  --version=2.0.0 \
  --title="Aurora Release"

# Mark as prerelease
soroban-registry changelog push \
  --contract-id=<uuid> \
  --version=2.0.0-rc.1 \
  --prerelease
```

---

## API Reference

### `GET /api/contracts/:id/changelog`

Returns the release history for a contract.

**Query Parameters:**

| Parameter | Type | Default | Description |
|---|---|---|---|
| `limit` | integer | `20` | Max releases to return (max 100) |
| `offset` | integer | `0` | Pagination offset |
| `include_prereleases` | boolean | `false` | Include prerelease versions |

**Response:**

```json
{
  "contract_id": "550e8400-e29b-41d4-a716-446655440000",
  "total_releases": 5,
  "releases": [
    {
      "version": "2.0.0",
      "title": "Aurora Release",
      "release_date": "2026-02-22T00:00:00Z",
      "is_prerelease": false,
      "breaking_changes": [
        {
          "change_type": "feat",
          "scope": "api",
          "description": "remove v1 endpoints",
          "commit_hash": "abc1234",
          "is_breaking": true,
          "author": "alice"
        }
      ],
      "features": [...],
      "fixes": [...],
      "other": [...],
      "markdown": "## [2.0.0] - 2026-02-22\n\n### ⚠ BREAKING CHANGES\n..."
    }
  ]
}
```

### `POST /api/contracts/:id/changelog`

Generate and store a changelog from raw commits.

**Request Body:**

```json
{
  "contract_id": "my-contract-id",
  "version": "2.0.0",
  "title": "Aurora Release",
  "is_prerelease": false,
  "commits": [
    { "hash": "abc1234def5678", "message": "feat(api)!: remove v1 endpoints", "author": "alice" },
    { "hash": "def5678abc1234", "message": "fix(auth): correct token expiry", "author": "bob" }
  ]
}
```

**Response (201 Created):**

```json
{
  "changelog_id": "660e8400-e29b-41d4-a716-446655440000",
  "version": "2.0.0",
  "entries_count": 2,
  "breaking_changes_count": 1,
  "skipped_commits": 0,
  "markdown": "## [2.0.0] - 2026-02-22\n...",
  "version_recommendation": {
    "current_version": "1.5.0",
    "recommended_version": "2.0.0",
    "bump_type": "major",
    "has_breaking_changes": true,
    "breaking_count": 1,
    "feature_count": 1,
    "fix_count": 1
  }
}
```

**Error Responses:**

| Status | Error Code | When |
|---|---|---|
| 400 | `InvalidVersion` | Version string is not valid semver |
| 400 | `NoValidCommits` | No commits match conventional commit format |
| 400 | `VersionBumpRequired` | Breaking changes present but major version not bumped |
| 404 | `ContractNotFound` | Contract UUID does not exist |
| 409 | `ChangelogExists` | Changelog for this version already exists |

---

## Version Bump Enforcement

The system enforces semantic versioning contracts:

| Change Type | Required Bump | Example |
|---|---|---|
| Breaking change (`!` or `BREAKING CHANGE:`) | **Major** | `1.5.0` → `2.0.0` |
| New feature (`feat`) | **Minor** | `1.5.0` → `1.6.0` |
| Bug fix, refactor, docs, etc. | **Patch** | `1.5.0` → `1.5.1` |

When the CLI or API detects a mismatch (e.g., breaking changes with only a minor bump), it:

- **CLI `generate`:** Prints a warning with the recommended version
- **CLI `validate`:** Exits with a non-zero code and error message
- **API `POST`:** Returns HTTP 400 with `VersionBumpRequired`

---

## Database Schema

### `contract_changelogs`

| Column | Type | Description |
|---|---|---|
| `id` | UUID (PK) | Changelog release ID |
| `contract_id` | UUID (FK → contracts) | Parent contract |
| `version` | TEXT | SemVer version string |
| `title` | TEXT | Optional release title |
| `release_date` | TIMESTAMPTZ | Release timestamp |
| `is_prerelease` | BOOLEAN | Whether this is a prerelease |
| `markdown` | TEXT | Rendered markdown content |
| `metadata` | JSONB | Commit counts, parser stats |

### `changelog_entries`

| Column | Type | Description |
|---|---|---|
| `id` | UUID (PK) | Entry ID |
| `changelog_id` | UUID (FK → contract_changelogs) | Parent changelog |
| `change_type` | ENUM | Conventional commit type |
| `scope` | TEXT | Optional scope |
| `description` | TEXT | Change description |
| `commit_hash` | TEXT | Git commit SHA |
| `is_breaking` | BOOLEAN | Whether this is a breaking change |
| `author` | TEXT | Commit author |

---

## Generated Markdown Format

The output follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - Aurora Release - 2026-02-22

### ⚠ BREAKING CHANGES

- **api:** remove v1 endpoints (abc1234) — alice

### Features

- **storage:** add IPFS pinning support (def5678) — bob
- **auth:** add OAuth2 provider (1234567) — carol

### Bug Fixes

- **indexer:** correct block height tracking (7654321) — dave

### Performance Improvements

- **query:** optimize contract search (aabbcc1) — eve
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  changelog:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for changelog

      - name: Generate changelog
        run: soroban-registry changelog generate --output=CHANGELOG.md --version=${{ github.ref_name }}

      - name: Validate version
        run: soroban-registry changelog validate ${{ github.ref_name }}

      - name: Push to registry
        run: |
          soroban-registry changelog push \
            --contract-id=${{ secrets.CONTRACT_ID }} \
            --version=${{ github.ref_name }}
        env:
          SOROBAN_REGISTRY_API_URL: https://api.soroban-registry.io
```

### Pre-release Hook

```bash
#!/bin/sh
# .git/hooks/pre-push — validate version before pushing tags
TAG=$(git describe --tags --exact-match 2>/dev/null)
if [ -n "$TAG" ]; then
  soroban-registry changelog validate "$TAG" || exit 1
fi
```
