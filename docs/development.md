# Developing iwafune

This page is for people who build or change iwafune itself. For using it, see the [README](../README.md).
The rules for contributions (structure, coding conventions, workflow) are in [CONTRIBUTING.md](../CONTRIBUTING.md); this page does not repeat them.

## Building from source

Requirements: Rust 1.100 or later (edition 2024), Node.js 24 or later, pnpm.

```bash
pnpm -C web install && pnpm -C web build                       # build the GUI (the fune binary embeds web/dist when it is compiled)
cargo run -p fune-cli -- -C fixtures/demo-6u gui                # open the GUI on the demo 6U satellite
cargo run -p fune-cli -- -C fixtures/demo-6u part ls            # list parts from the CLI
cargo run -p fune-cli -- -C fixtures/demo-6u analyze power.energy_balance
```

## Repository layout

| Path | Contents |
|---|---|
| `crates/lib/units` (`sat-units`) | Units, dimensions and typed quantities |
| `crates/lib/astro` (`sat-astro`) | Math, integrators, time, orbits, ephemerides, eclipses, Earth models, attitude laws, radiation belts and shielding |
| `crates/lib/checkkey` (`sat-checkkey`) | The key format of analysis checks, shared by the analyses and the model |
| `crates/lib/analysis` (`sat-analysis`) | Subsystem analyses with typed SI inputs and outputs; no knowledge of the design model |
| `crates/app/model` (`fune-model`) | Design data types, built-in vocabulary, resolution and validation |
| `crates/app/store` (`fune-store`) | Reading and writing the TOML directory, revisions, locking |
| `crates/app/ops` (`fune-ops`) | `Op`, the single entry point for edits: plan, apply, and conversion to CLI commands |
| `crates/app/projection` (`fune-projection`) | Resolved design → analysis inputs, with a list of missing inputs |
| `crates/app/service` (`fune-service`) | Every function the CLI and the HTTP API call, plus their types (exported to TypeScript) |
| `crates/app/server` (`fune-server`) | HTTP API (axum) and the embedded GUI |
| `crates/app/report` (`fune-report`) | Report export: the data model of tables and figures, the SVG charts and the Typst module |
| `crates/app/cli` (`fune-cli`) | The `fune` command |
| `crates/xtask` (`xtask`) | `cargo xtask check`: the architecture rules |
| `web/` | GUI (React, TypeScript) |
| `fixtures/demo-6u` | Demo 6U satellite |
| `docs/` | User guide (`guide/`), analyses (`analyses/`), data model, architecture and registry, report export, trapped-radiation data, roadmap; Japanese editions in `ja/` |
| `site/` | Documentation site (Astro, Starlight) that renders `docs/` |

`crates/lib/*` never depend on `crates/app/*`. `cargo xtask check` enforces the dependency direction and the
"one capability, one place" rules recorded in [architecture/registry.md](architecture/registry.md).
Every edit is an `Op`, and the CLI and the HTTP API call the same service functions; a parity test checks that the same edit
made through either writes byte-identical files. The overall design is in [architecture.md](architecture.md).

## Development commands

```bash
cargo xtask check                              # dependency and registry rules
cargo test --workspace
cargo clippy --all-targets -- -D warnings
pnpm -C web lint && pnpm -C web typecheck && pnpm -C web test
cargo test -p fune-service export_ts           # regenerate web/src/api/gen after changing API types
```

## Documentation site

The site at <https://akira-nkjm.github.io/iwafune/> is built from `docs/` by `site/` (Astro with Starlight). It is a separate pnpm project
(Node 24 or later); the content is never copied, so editing a Markdown file in `docs/` is all it takes to change a page.

```bash
pnpm -C site install --frozen-lockfile
pnpm -C site dev                  # http://localhost:4321/iwafune/ (reads docs/ live)
pnpm -C site build                # static site in site/dist
pnpm -C site check                # astro check (types)
pnpm -C site lint && pnpm -C site test
pnpm -C site check:dist           # after a build: links under /iwafune/, anchors, no external resources, search index
```

How `docs/` maps to routes:

- `docs/guide/install.md` is `/guide/install/`, `docs/guide/index.md` is `/guide/`, `docs/ja/guide/install.md` is `/ja/guide/install/`.
  English is the root locale; a page without a Japanese edition is shown in English under `/ja/` with Starlight's notice.
- A page has no front matter. The first `# H1` is the title (and is removed from the body), the first paragraph after it is the description.
- Relative links between `.md` files become site routes (with their `#anchor`). Relative links that leave `docs/`
  (`../../crates/…`) become links to the file on GitHub (`main`). Images are served from `/docs-files/…`.
- Math is GitHub style: `$…$`, `$$…$$` and a `math` fence (KaTeX). A `mermaid` fence is drawn in the browser, following the light and dark theme.
  `> [!NOTE]` style alerts become asides.
- A screenshot placeholder `<!-- screenshot: /design/components — description -->` shows the description in a frame. If
  `site/src/assets/shots/<slug>.png` exists (`<slug>.ja.png` for Japanese pages), the image is shown instead.
  The slug is the route in lower case with every run of characters other than letters and digits replaced by `-`
  (`/design/power/budget` is `design-power-budget`, `/` is `home`).
- The sidebar lists the pages that exist, in a fixed order (`site/src/lib/sidebar.ts`); a page that is not listed there is still built.
- The landing page (`/`, `/ja/`) and the documentation entry (`/docs/`, `/ja/docs/`) are Astro components with English and Japanese copy in `site/src/i18n/landing.ts`.
- Colours, fonts and sizes are only in `site/src/styles/tokens.css`.

To capture screenshots from the demo, start a GUI of your own (`fune -C fixtures/demo-6u gui --no-open --port <port>`) and run
`CHROMIUM_PATH=/path/to/chrome pnpm -C site shots --base http://localhost:<port>` (add `--lang ja` for Japanese). It reads the placeholders
in `docs/`, visits each route at 1440 x 900 at 2x, waits until the loading indicators are gone, and writes `site/src/assets/shots/<slug>.png`.
Do not use ports 3000 or 5173 for this.

Pull requests that change `site/` or `docs/` run the `site` job of CI. Pushes to `main` publish the site with `.github/workflows/pages.yml`
(Pages must be set to "GitHub Actions" in the repository settings).

CI runs the architecture check, the Rust checks (format, clippy, tests including the parity tests and the generated-types check) and the web checks (lint, typecheck, test, build). On pushes to `develop` and `main` it also builds the release binary and smoke-tests the CLI and the HTTP server.

## Branches and releases

Daily work goes to `develop`. `main` only receives pull requests from `develop`, and each one is a release: bump
the version on `develop` first (`[workspace.package] version` in `Cargo.toml` and `version` in `web/package.json`,
to the same value). [release.yml](../.github/workflows/release.yml) checks on the pull request that the version is
consistent and not released yet; after the merge it publishes the container image ([image.yml](../.github/workflows/image.yml)),
then creates the tag `vX.Y.Z` and its GitHub Release. Tags are not pushed by hand.
