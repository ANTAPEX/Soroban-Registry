---
name: frontend-ui
description: House design rules for the Soroban Registry frontend. Read before writing or changing any UI under frontend/ - components, pages, charts, styling, layout or copy. Says which design system already exists here, which tokens to use instead of raw colour values, and when the general-purpose design skills apply and when they do not.
---

# Soroban Registry frontend

This is not a blank canvas. `frontend/` is a shipped Next.js 15 app with a deliberate
visual identity, so the job is almost always to extend that identity rather than to
invent one. Read this file first, then reach for a general design skill only where
this one leaves the question open.

## The design language that already exists

`frontend/app/globals.css` calls it the **Stellar editorial design system** and models it
on Stellar.org's public brand language:

- A black and white base with one signature warm-gold accent.
- An editorial serif display face over a clean grotesque for everything else.
- Light and dark are both first-class, not a light theme with a dark afterthought.

Treat that as settled. It is a deliberate brand match, so do not "fix" it toward a
different look, and do not restyle a shipped page because a newer aesthetic seems
better. If you believe the direction itself is wrong, say so and let a human decide
before writing code.

Two consequences that regularly surprise people: the hero uses an animated aurora
gradient and a gold dot field, and several surfaces use glassmorphism and glow. A
general design skill will read those as generated-looking defaults. Here they are
intentional, they match the brand, and they stay.

## Colour: use the tokens, never a raw hex

Every colour is a CSS custom property defined twice in `app/globals.css`, once under
`:root` and once under `.dark`, and exposed to Tailwind through `@theme inline`. So
`bg-background`, `text-foreground`, `border-border`, `bg-card`, `text-muted-foreground`
and `bg-primary` all work as ordinary utilities and both themes follow automatically.

A hardcoded hex value in a component is a bug, because it only has one value and this
app has two themes. The check:

```bash
grep -rn "#[0-9a-fA-F]\{6\}" frontend/app frontend/components --include='*.tsx'
```

That currently returns matches, and most of them are chart series colours where Recharts
needs a literal value. If you need a literal for a chart, read it from a shared palette
module and make sure it has a dark-mode counterpart. Do not add a new inline hex to a
component.

The full token list lives at the top of `app/globals.css`. Read it rather than guessing
a name.

## Type

Two families, loaded through `next/font/google` in `app/layout.tsx` and wired to CSS
variables:

- `--font-fraunces` (Fraunces, serif) for display type. `h1` through `h4` already use it
  sitewide via a base-layer rule, so a heading needs no font class.
- `--font-inter` (Inter, sans) for body and UI.

Do not introduce a third family, and do not set a font on a heading to get the serif:
it is already there.

## Dark mode

Dark mode is class-based: `@variant dark (&:where(.dark, .dark *))`, toggled by
`components/ThemeToggle.tsx`. Any new surface must be checked in both themes before it
ships. Using tokens gets you this for free; the moment you reach for a literal colour,
you own both themes by hand.

## Motion

Motion that answers a user action is welcome. Ambient motion is not: it should be rare,
deliberate and reducible.

`@media (prefers-reduced-motion: reduce)` in `app/globals.css` currently covers only the
four `animate-fade-in-up` classes. The 12-second `aurora-shift` loop, the skeleton pulse
and the global transition on `*` are not covered. So if you add an animation, add its
reduced-motion case in the same change, and prefer extending the existing block to
adding a new one.

## Writing

Interface copy is design content. Name things the way a contract author would say them,
not the way the API does. A button says what happens when it is pressed, and the same
action keeps the same word through the whole flow. Errors say what went wrong and what
to do next. Sentence case throughout.

There is a CI check that fails on emoji in any changed file, and it scans whole files
rather than changed lines, so touching a file pulls its pre-existing glyphs into scope.
Keep emoji out of components, copy and documentation.

## When the general design skills apply

Two plugins are enabled for this repository, in `.claude/settings.json`:

- **`frontend-design`** (Anthropic). General guidance on aesthetic direction, typography
  and avoiding templated defaults. Useful on any new surface. Its advice to choose
  distinctive typefaces and a fresh palette is already spent here, so apply the rest of
  it (hierarchy, restraint, quality floor, copy) inside the system above.
- **`taste-skill`** (MIT, Leonxlnx). A library of design skills. Its default skill states
  its own scope plainly: landing pages, portfolios and redesigns, and explicitly not
  dashboards, data tables or multi-step product UI. Most of this app is exactly what it
  excludes, so it fits the marketing surfaces and the homepage hero, not the contract
  browser, the analytics pages, the dependency graph or the compatibility matrices. Its
  `redesign-skill` is the one to reach for on existing screens, because it audits before
  it changes anything.

Neither of them knows about the tokens above. This file wins where they disagree.

## Before you push a frontend change

`tsc` and `eslint` passing does not mean the app builds. Run the production build:

```bash
cd frontend
npm ci --legacy-peer-deps   # plain npm ci fails: Redux Toolkit 1.x peers React <= 18
npx tsc --noEmit
npx jest --runInBand
npx next build
```

`next build` catches a class of error the other two miss. The commonest one here: a
`"use client"` directive must be the first statement in the file, and an import inserted
above it silently turns the file into a Server Component. Check with:

```bash
cd frontend
for f in $(grep -rl '"use client"' --include='*.ts*' app components hooks providers); do
  [ "$(head -1 "$f")" = '"use client";' ] || echo "MISPLACED: $f"
done
```

`package-lock.json` is the only lockfile and that is deliberate. `vercel.json` pins the
install command so the deployed tree is the tree CI tested. Do not add a second lockfile.
