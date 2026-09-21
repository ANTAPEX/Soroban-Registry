# Contributing to Soroban Registry

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing to the Soroban Registry project.

## Table of Contents

- [Getting Started](#getting-started)
- [Feature Branch Naming](#feature-branch-naming)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Code Standards](#code-standards)
- [Issue Selection](#issue-selection)

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/Soroban-Registry.git
   cd Soroban-Registry
   ```
3. **Add upstream remote** to keep your fork in sync:
   ```bash
   git remote add upstream https://github.com/ALIPHATICHYD/Soroban-Registry.git
   ```
4. **Create a feature branch** (see below for naming conventions)

## Feature Branch Naming

When working on an issue, always create a feature branch using this format:

```bash
git checkout -b feature/issue-#-kebab-case-title
```

### Naming Convention Rules

- **Prefix**: Always start with `feature/`
- **Issue number**: Include `issue-#` (where # is the GitHub issue number)
- **Description**: Use kebab-case (lowercase with hyphens, no spaces)
- **Length**: Keep it concise but descriptive (under 50 characters)

### Examples

| Issue | Title                                         | Branch Name                                      |
| ----- | --------------------------------------------- | ------------------------------------------------ |
| #37   | Implement Contract Dependency Tracking System | `feature/issue-37-contract-dependency-tracking`  |
| #45   | Create Contract Backup System                 | `feature/issue-45-contract-backup-system`        |
| #59   | Create Contract Governance Framework          | `feature/issue-59-contract-governance-framework` |
| #2    | Implement Contract Sorting Options            | `feature/issue-2-contract-sorting-options`       |
| #10   | Add Contract Interaction History Tracking     | `feature/issue-10-interaction-history-tracking`  |

### Branch Types

While most contributions use `feature/`, you may also use:

- `fix/` - For bug fixes
- `docs/` - For documentation improvements
- `refactor/` - For code refactoring
- `test/` - For testing improvements

Example: `fix/issue-42-rate-limiting-bug` or `docs/issue-15-api-documentation`

## Development Setup

The [README](README.md) has the full walkthrough, including the three environment
variables `.env.example` is missing. The short version:

### Backend (Rust)

```bash
cd backend

# Build. No database is needed at compile time.
cargo build

# Run tests. Note: the api crate's test targets do not compile on main right
# now, so this fails before running anything. The library builds clean.
cargo test

# Run a service. The three binaries are api, indexer and seeder;
# verifier and shared are libraries.
cargo run --bin api
cargo run --bin indexer
cargo run --bin seeder -- --count 50
```

### Frontend (Next.js)

```bash
cd frontend

# Install dependencies. --legacy-peer-deps is required: @reduxjs/toolkit 1.9
# declares a peer of React 18 and this app is on React 19.
npm ci --legacy-peer-deps

# Run development server
npm run dev

# Build for production. Run this before every push -- tsc and eslint both miss
# errors that the Vercel build catches.
npm run build
```

### CLI (Rust)

`cli/` is a standalone Cargo package, not a workspace member, so `cargo` from the
repository root will not find it.

```bash
cargo build --manifest-path cli/Cargo.toml --locked --all-targets
cargo run --manifest-path cli/Cargo.toml --bin soroban-registry -- --help
```

### Database

```bash
createdb soroban_registry
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"
```

That is usually all you need: the API applies every migration in
`database/migrations/` itself on startup. To apply them by hand instead, install
the SQLx CLI and run `sqlx migrate run --source database/migrations` from the
repository root.

## Making Changes

### Before You Start

1. **Check the issue description** for requirements and acceptance criteria
2. **Review related code** to understand the codebase structure
3. **Create a branch** using the naming convention above
4. **Keep commits atomic** - one logical change per commit

### Commit Message Format

Use clear, descriptive commit messages:

```
[type] Short description (50 chars max)

Longer explanation if needed. Explain what and why, not how.

Closes #[issue-number]
```

**Types**: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`

**Example**:

```
feat: Add contract dependency tracking API endpoint

Implements GET /contracts/{id}/dependencies endpoint that returns
the dependency tree for a contract. Includes semver constraint validation
and circular dependency detection.

Closes #37
```

### Code Style

#### Rust

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document public APIs with doc comments

```rust
/// Fetches contract dependencies
///
/// # Arguments
/// * `contract_id` - The contract ID to fetch dependencies for
///
/// # Returns
/// A vector of contract dependencies
pub fn get_dependencies(contract_id: &str) -> Result<Vec<Dependency>> {
    // implementation
}
```

#### TypeScript/JavaScript

- Use ESLint configuration provided in `frontend/.eslintrc`
- Use Prettier for formatting
- Write meaningful type annotations
- Document complex logic with comments

```typescript
/**
 * Fetches contract dependencies with semver constraint validation
 * @param contractId - The contract ID
 * @returns Promise resolving to dependency array
 */
const getDependencies = async (contractId: string): Promise<Dependency[]> => {
  // implementation
};
```

#### SQL

- Use snake_case for table/column names
- Add comments for complex queries
- Include migration version in filenames (e.g., `002_add_dependencies.sql`)

```sql
-- Add dependencies table for tracking contract relationships
CREATE TABLE contract_dependencies (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  contract_id UUID NOT NULL REFERENCES contracts(id),
  depends_on_id UUID NOT NULL REFERENCES contracts(id),
  version_constraint TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p api

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_dependency_validation
```

### Frontend Tests

```bash
cd frontend

# Run tests
npx jest --runInBand

# Watch mode
npm run test:watch

# With coverage (this is what `npm test` does)
npm test
```

Five suites fail on `main` and are not your fault: `__tests__/lib/api.test.ts`,
`ipfsMirror`, `contractsContentFilters`, `components/FilterPanel` and `resilience`.
Baseline your branch against a clean `main` rather than expecting green.

### Integration Tests

- Add integration tests to the crate they exercise: `backend/api/tests/`, `backend/indexer/tests/`, `backend/shared/tests/` or `backend/registry_client/tests/`. There is no top-level `backend/tests/`.
- Name tests clearly: `test_contract_dependency_api.rs`
- Test edge cases and error conditions

### Manual Testing

1. **Start the API**: `cargo run --bin api` from `backend/`
2. **Start the frontend**: `npm run dev` from `frontend/`
3. **Test the feature** using the UI or API endpoints
4. **Check different networks** (Mainnet, Testnet, Futurenet)

## Submitting a Pull Request

### Before Submitting

- [ ] Code follows project style guidelines
- [ ] Tests pass locally (`cargo test`, `npx jest --runInBand`)
- [ ] No console errors or warnings
- [ ] Documentation is updated if needed
- [ ] Commits are atomic and well-messaged
- [ ] Branch is up-to-date with `main`

### Sync with Main

```bash
git fetch upstream
git rebase upstream/main

# If conflicts, resolve them, then:
git add .
git rebase --continue
```

### Push and Create PR

```bash
git push origin feature/issue-#-your-branch-name
```

Then open a PR on GitHub with:

**Title**: Match the issue title or be descriptive

```
Implement Contract Dependency Tracking System
```

**Description**: Use this template

```markdown
## Description

Brief explanation of what this PR does.

## Changes

- List major changes
- For example: Added GET /contracts/{id}/dependencies endpoint
- Connect to database schema changes if any

## Testing

- [ ] Unit tests added/updated
- [ ] Manual testing completed
- [ ] All tests passing

## Related Issues

Closes #37

## Screenshots (if applicable)

Include UI changes or diagrams if relevant.

## Checklist

- [ ] Code follows style guidelines
- [ ] Tests added for new functionality
- [ ] Documentation updated
- [ ] No breaking changes
```

### Key Rules for PRs

1. **Link to the issue**: Always include `Closes #[issue-number]` in the PR description
2. **One feature per PR**: Keep PRs focused and reviewable
3. **Keep it small**: Aim for 200-500 lines of changes when possible
4. **Be responsive**: Engage with reviewers promptly
5. **Update based on feedback**: Address all review comments

## Code Standards

### Rust Code Standards

```bash
# Format your code
cargo fmt

# Check for issues
cargo clippy -- -D warnings

# Run tests
cargo test
```

### Frontend Code Standards

```bash
# Lint
npx eslint . --fix

# Type check
npx tsc --noEmit

# Test
npx jest --runInBand
```

### Documentation

- Update README if adding new features
- Add code comments for complex logic
- Document API endpoints with request/response examples
- Include CLI usage examples for new commands

## Issue Selection

### Finding an Issue

1. Look for issues labeled with your skill level:
   - `good-first-issue` - Perfect for newcomers
   - `medium` - Intermediate complexity
   - `hard` - Advanced complexity

2. Check the issue status:
   - No one is assigned? You can claim it
   - Already assigned? Ask in comments or pick another

3. Read the issue thoroughly:
   - Understand the requirements
   - Review acceptance criteria
   - Check for any blockers or dependencies

### Claiming an Issue

Comment on the issue:

```
I'd like to work on this. Can I be assigned?
```

An maintainer will assign you or guide you further.

### Work in Progress

If you need time, create a draft PR:

```bash
git push origin feature/issue-#-your-branch

# Create draft PR on GitHub, or use GitHub CLI:
gh pr create --draft
```

## Getting Help

- **Questions?** Ask in the issue comments or discussions
- **Stuck?** Create a draft PR and ask for guidance
- **Bug in main?** Create a new issue or discuss with maintainers
- **Design decisions?** Discuss in the issue before starting

## Recognition

Contributors are recognized in:

- The [README.md](README.md) contributors section (for significant contributions)
- Git commit history
- GitHub contributors page
- Release notes (for major features)

---

Thank you for contributing! We appreciate your effort in making Soroban Registry better.
