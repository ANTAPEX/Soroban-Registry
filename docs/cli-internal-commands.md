# Internal-only CLI commands (#1177)

Debug and diagnostic subcommands (scratch API wiring probes, internal state
dumps) must never reach real users. Convention:

1. Gate the variant: `#[cfg(feature = "internal-debug")]` on the `Commands`
   enum variant in `cli/src/cli.rs`, with an explicit `#[command(name = ...)]`.
2. Gate the arm: same `#[cfg(...)]` on the `dispatch.rs` match arm.
3. Add the `internal-debug` cargo feature in `cli/Cargo.toml` (already present).
4. Never add `internal-debug` to default features or release profiles.

CI (`cli-ci.yml`, "Internal-debug gate check") fails if `debug-dump`
appears in a default `--help`, or is missing from an
`--features internal-debug` build. Reference implementation: `debug-dump`.
