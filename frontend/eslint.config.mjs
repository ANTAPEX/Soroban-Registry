import { dirname } from "path";
import { fileURLToPath } from "url";
import { FlatCompat } from "@eslint/eslintrc";
import tsPlugin from "@typescript-eslint/eslint-plugin";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const compat = new FlatCompat({
  baseDirectory: __dirname,
});

/** @type {import('eslint').Linter.Config[]} */
const eslintConfig = [
  // Ignore non-source directories
  {
    ignores: [
      ".next/**",
      "out/**",
      "build/**",
      "coverage/**",
      "next-env.d.ts",
      "**/*.stories.*",
      "**/.storybook/**",
      "node_modules/**",
    ],
  },
  // eslint-config-next already bundles @typescript-eslint/parser + react rules
  ...compat.extends("next/core-web-vitals"),
  // Register @typescript-eslint plugin so inline disable comments don't cause hard errors
  {
    plugins: {
      "@typescript-eslint": tsPlugin,
    },
    rules: {
      // The base rule must be off wherever its TypeScript counterpart is on:
      // it only understands runtime bindings, so it reports every parameter
      // name in a type-only position as unused. On this codebase that was 125
      // false positives -- interface method signatures (types/realtime.ts),
      // ambient declarations (types/webxr.d.ts, lib/analytics.ts) and
      // constructor parameter properties (lib/errors.ts), which TypeScript
      // turns into class fields -- plus 34 duplicates of warnings the
      // @typescript-eslint rule below already reports. That rule reads the
      // same options and handles all of those correctly.
      "no-unused-vars": "off",
      "no-console": "warn",
      // TypeScript-specific rules — set to warn so inline disable comments are valid
      "@typescript-eslint/no-unused-vars": [
        "warn",
        {
          vars: "all",
          args: "after-used",
          ignoreRestSiblings: true,
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/ban-ts-comment": "warn",
      "@typescript-eslint/no-require-imports": "warn",
    },
  },
];

export default eslintConfig;
