# `.claude/`

Configuration that applies to anyone working on this repository with Claude Code.

## `skills/frontend-ui/SKILL.md`

The house design rules for `frontend/`: which design system already exists, which CSS
tokens to use instead of raw colour values, how dark mode works, and which checks to run
before pushing a frontend change. It is picked up automatically, with no install step.

Edit it as the frontend changes. It is repository content, not a vendored dependency.

## `settings.json`

Registers two plugin marketplaces and enables one design plugin from each:

| Plugin | Source | Licence | What it gives you |
| --- | --- | --- | --- |
| `frontend-design` | [anthropics/claude-code](https://github.com/anthropics/claude-code/tree/main/plugins/frontend-design) | Anthropic Commercial Terms | Guidance on aesthetic direction, typography and avoiding templated defaults |
| `taste-skill` | [Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill) | MIT | A library of design skills: anti-slop frontend, redesign audits, minimalist, brutalist, image-to-code and others |

Neither is vendored into this repository. Both are fetched from their own repositories,
so they stay current and their licences stay with their authors.

### One-time setup

Registering a marketplace does not install its plugins, so each person runs this once
after trusting the repository folder:

```bash
claude plugin install frontend-design@claude-code-plugins --scope project
claude plugin install taste-skill@taste-skill --scope project
```

Then `/reload-plugins`, or start a new session.

Both are general-purpose and know nothing about this codebase. `skills/frontend-ui`
is what grounds them here, and it wins wherever they disagree with it.
