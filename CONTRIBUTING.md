# iwafune development rules

A foundation for studying the conceptual design of satellites. Design data (a git-managed directory of TOML) is edited from both the CLI (`fune`) and the GUI, and the analyses of each subsystem are run on it.
The overall design is in [docs/architecture.md](docs/architecture.md), the shape of the data is in [docs/model.md](docs/model.md), where each feature lives is in [docs/architecture/registry.md](docs/architecture/registry.md),
and upcoming features are in [docs/roadmap.md](docs/roadmap.md) and GitHub Issues.
Building from source, the repository layout and the release flow are in [docs/development.md](docs/development.md).

## Structural rules (most important)

1. **The registry (registry.md) is the authority on where a feature lives.** Before creating a new type, function or module, check the registry. If the same feature already exists, use it.
   If it does not, add one line to the registry before you build it, and add one line to the change history in §5 as well. Do not add features anywhere the registry does not list.
2. **Each feature lives in exactly one place (the "single home" rule).** Do not write any of the following in two places: integrators, celestial body positions, eclipses, coordinate transforms, orbit propagation, unit parsing, conversion and display, default values, verdict criteria, display names of vocabularies,
   how paths and IDs are read, and so on. If you find something similar, discuss it before you build it.
   When two crates have no choice but to hold the same value (because the dependency direction prevents sharing it), confirm in a test of a crate that can see both that the two agree.
3. **Do not merge things with different responsibilities.** Computation and display, loading and checking, interpreting an Op and writing a file go in separate functions and modules.
   As a guideline, keep a file to 400 lines and a function to 60 lines (data such as coefficient tables excepted).
4. **Keep the dependency direction.** `crates/lib/*` must not depend on `crates/app/*`. Modules inside a crate must not form cycles either (split the shared part out into a leaf module).
5. **Parity between the CLI and the GUI.** Every operation is built as a `fune-service` function and called from both the CLI and HTTP. The interpretation of strings a person types (such as the value type for `fune set`) is
   also owned by the service, and the GUI goes through the same function (`POST /ops/from_text`). The same edit made through the CLI and through HTTP must produce files that are byte-for-byte identical (parity test).
   The only exceptions are `fune init` (the server only runs inside a project, so it is not available over HTTP), `fune self-update` (it replaces the CLI binary itself, which a server cannot do for the person running it) and `fune skill` (it writes an agent instruction file into the caller's directory, not design data, and a server cannot write there; it is not a design-file write, so it does not touch `fune_store::write::commit`).
   Writes to design files also go only through Ops (only store and ops call `fune_store::write::commit`). The exceptions are `fune fmt` (rewriting into canonical form) and importing a shared catalog (the derived `catalog.lock`); `cargo xtask check` verifies these two.

`cargo xtask check` mechanically verifies: the dependency direction, module cycles, that every crate is in the registry's crate table, the "single home" rule (`crates/xtask/src/single_home.rs`; a definition rule that matches nothing under its allowed paths is itself an error; propellant mass flow `F / (Isp · g₀)` only in `sat_astro::rocket`),
the Web prohibitions (`fetch(` only in the API client; number formatting and `Intl.NumberFormat` only in `web/src/quantity/`; hard-coded colors (hex values and color names such as `white`) and color mixing (`color-mix(`) only in `lib/tone.ts` (the colors in `index.css` are generated from it with `pnpm -C web tokens`; `crates/xtask/src/web_colors.rs`);
no negative letter-spacing `tracking-tight`; `animate-pulse` only in the ui skeleton (`web/src/ui/skeleton.tsx`); no budget-kind literal passed as an argument (`budget("mass")`); `queryKey: [` only in the query-key files (`web/src/api/keys.ts`, `web/src/quantity/format-batch.ts`, `web/src/kit/feedback/Settled.tsx`); no splitting IDs or paths on `/` or `.`; no turning typed strings into numbers with `Number(` or `parseFloat(` (`crates/xtask/src/web_strings.rs`);
no reading of JSON by field name in the report export (`serde_json`, `.get("…")`, `.pointer("…")` and `x["…"]` are errors in `crates/app/service/src/report*`; read the typed analysis outputs, so a renamed field fails to compile instead of leaving a table empty; a rule that finds no report file is itself an error; `crates/xtask/src/report_json.rs`),
no comparing a role with a string (`p.role === "battery"`; `crates/xtask/src/web_roles.rs`) and no comparing a subsystem identifier with a string (`system === "power"`; the API returns the judgement as a field, and everything specific to one subsystem (dedicated tabs, extra tabs, page actions, extras, preloaded chunks) is registered in the single typed table `web/src/features/systems/dedicated.tsx`; no other table keyed by subsystem (`Record<System, …>`) is allowed except the colour and icon table `lib/systems.ts`; `crates/xtask/src/web_systems.rs`);
skeletons (`ui/skeleton`) only in `kit/feedback/Loading.tsx`; the animated logo mark for waiting (`MarkLoader`, `mark-glow`) only in `kit/feedback/` (screens use `Working`, `Refreshing` or `Refreshable`); dimming content while refreshing (`opacity-55`) only in `kit/feedback/` (screens use `Refreshable`, or `refreshingDim` for an element that cannot be wrapped, such as a table row); no hand-written lists of subsystems or budget kinds (use the API's `DefsView.systems` and `budget_kinds`); page URLs (such as `/design/…`) built only with `paths` in `lib/nav.ts`; `features/` may be imported only by
`features/` and `routes/`, and imports between page contents are allowed only if they are in the allowlist with a stated reason (`FEATURE_EDGES`) (`crates/xtask/src/web_pages.rs`)), Japanese in Rust string literals outside comments and tests, except the allowlisted files that hold bilingual name pairs (`crates/xtask/src/english_text.rs`), and a warning (not an error) for files over 400 lines and Rust functions over 60 lines (`crates/xtask/src/sizes.rs`). When you add a rule, add it to xtask too.

## Layer responsibilities

| crate | Responsibility | Must not |
|---|---|---|
| `sat-units` | The single source of truth for units, dimensions and quantities. Typed quantities (`Qty`), display kinds (`Fraction`, `Decibel*`), display strings for quantities and units (`format`) | — |
| `sat-astro` | Mathematics, integrators, time, orbits, celestial bodies, eclipses, the Earth, gravity, geomagnetism, atmosphere, ground stations, attitude laws, time series, the rocket equation. Conventional defaults (`with_defaults` and the like) | Analysis-specific decisions, file I/O (large data is received as bytes) |
| `sat-analysis` | Analyses and verdicts for each subsystem. Inputs and outputs are typed SI quantities. Modeling defaults belong to the analysis (inputs are `Option`) | Depend on `fune-*`, do I/O. Numbers in verdict text are written only with `spec::quantity_text` (sat-units) |
| `fune-model` | Design data types, built-in attributes, kinds and display names of vocabularies, resolution (Resolved), checking (check) | File I/O |
| `fune-store` | Reading and writing directory ⇄ Design (toml_edit, canonical form = `fune fmt`), revision, locks | Check logic, physics computation |
| `fune-ops` | Op (the only entry point for edits), plan / apply, to_cli | HTTP or display |
| `fune-projection` | Mapping Resolved to the input types of the analyses, and listing missing inputs | Physics computation, modeling defaults |
| `fune-service` | All the functions that the CLI and HTTP call, and their input and output types (TS generated with ts-rs). Display strings are built with `sat_units::format` and put into DTOs | Knowledge of HTTP, owning formatting rules itself. File I/O goes through store (the exceptions are the cache of external data and writing out TS) |
| `fune-server` | axum routing and JSON conversion, serving the SPA, the Host allowlist | Logic |
| `fune-cli` | Argument parsing and display (tables, colors, diagnostics) | Logic |
| `web/` | Display and interaction | Physics computation, dimensional computation, unit conversion, interpreting IDs or paths by splitting strings (the API returns them as structure), verdicts and aggregation (how a KPI's representative value is chosen, how verdicts are counted, fill ratios; the API returns these) |

## Coding conventions

- Rust: edition 2024, latest stable. Pass `cargo fmt` and `cargo clippy --all-targets -- -D warnings`.
- Write modules as `x.rs` and `x/y.rs`; do not use `mod.rs` (detected by clippy's `mod_module_files`; the only exception is the shared `tests/common/mod.rs` of integration tests).
- Errors: `thiserror` in libraries, `anyhow` in binaries. Do not use `unwrap` / `expect` in library code (allowed in tests).
- Write a description in `///` for public types and functions, and at the top of each crate's lib.rs list the modules it holds and their responsibilities. Code comments and `///` docs are written in Japanese; the exception is the `///` help text of clap arguments and commands, which is written in English because it is shown by `--help`.
- Language of user-visible text: everything the CLI and the libraries show to users (messages, diagnostics, verdict text, `--help`) is written in English only. `docs/` and `CONTRIBUTING.md` are in English. The display names in the demo data (`fixtures/`) are in English. The GUI keeps its Japanese and English dictionaries. Commit messages and PR titles are in English.
- Give each analysis function at least one test that compares against a hand calculation or a literature value. In a refactoring, confirm that the demo's analysis results do not change numerically.
- TypeScript: strict, Biome, no `any`. Use only `web/src/api/gen/` (generated by ts-rs) for API types, and do not write them by hand.
  When you change a type, regenerate it with `cargo test -p fune-service export_ts` and commit the result (CI checks the diff).
- Web UI: text goes in the i18n dictionaries (both Japanese and English). Show numbers and quantities with the API's formatting (`Formatted`). While loading, use the skeleton in `kit/feedback/Loading.tsx`.
  Typefaces are Noto Sans JP (monospace: Noto Sans Mono and BIZ UDGothic). Make the `className` of a child passed to Radix's `asChild` a string (a function breaks it).
- Pin dependencies to the latest stable version (one released at least two weeks ago). The package manager is pnpm. The only languages are Rust and TS.
- How to write TOML is in docs/model.md §10 (output stays within TOML 1.0). Edit the demo data (`fixtures/`) with the CLI too (`fune part place`, `fune set`, and so on),
  and make it pass `fune fmt --check`. The demo data is fictional; do not put in values of real parts or values from materials with distribution restrictions.

## Workflow

- Commit messages are in English. Day-to-day work goes into `develop` through pull requests (CI: architecture, Rust, web, bundle build. On a PR only the checks for the changed parts run, and the bundle build runs on pushes to develop and main). `develop` and `main` require the CI checks `changes`, `architecture`, `rust` and `web` to pass, so nothing can be pushed to `develop` directly.
- Tie work to an Issue. For each Issue, create a branch `issue-<number>-<short-name>` (for example `issue-21-port-refs`) from develop and
  merge it with a PR to develop. In the PR body, write `Closes #<number>` if it closes the Issue, or `Part of #<number>` if it is one stage of it.
  Also add `(#<number>)` at the end of the commit message subject. Small fixes without an Issue (such as a typo in a document) still go through a pull request (no Issue needed).
- `main` is for releases only. Changes enter it only through a PR from develop, and before the PR you bump the version on develop (set `[workspace.package] version` in `Cargo.toml` and
  `version` in `web/package.json` to the same value). On the PR, release.yml checks that "the versions match and it has not been released yet", and when it enters main it
  creates the container image (`ghcr.io/<owner>/fune:<version>`), the tag `v<version>`, and a GitHub Release. Do not create tags by hand.
- Commit only after all checks pass.

## Common commands

```bash
cargo xtask check                 # checks for dependencies, cycles, the registry, and the "single home" rule
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo test -p fune-service export_ts   # regenerate the TS types
cargo run -p fune-cli -- -C fixtures/demo-6u check
cargo run -p fune-cli -- -C fixtures/demo-6u fmt --check
pnpm -C web lint && pnpm -C web typecheck && pnpm -C web test && pnpm -C web build
```
