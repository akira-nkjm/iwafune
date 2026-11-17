<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
    <img src="docs/assets/banner-light.svg" width="480" alt="iwafune — Satellite design workbench">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/akira-nkjm/iwafune/actions/workflows/ci.yml"><img src="https://github.com/akira-nkjm/iwafune/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a>
  <img src="https://img.shields.io/badge/Rust-1.100-062654?logo=rust&logoColor=white" alt="Rust 1.100">
  <img src="https://img.shields.io/badge/React-19-3382c2?logo=react&logoColor=white" alt="React 19">
  <img src="https://img.shields.io/badge/design%20data-TOML%201.0-0097a7" alt="Design data in TOML 1.0">
</p>

<p align="center">
  <b>Design data in git</b> · <b>CLI and GUI as equals</b> · <b>16 subsystem analyses</b>
</p>

iwafune is a workbench for the conceptual design of small satellites. The design lives in a git-friendly
directory of TOML files. You edit it from either the `fune` CLI or a web GUI, and run analyses for power,
AOCS, C&DH, communications, thermal, structure and propulsion on the same data.

> The name comes from Ama-no-Iwafune, the rock boat said to have carried Nigihayahi down from the heavens.
> The logo is its hull.

## Highlights

|  |  |
|---|---|
| **Design data as files** | One entity per TOML file: parts, modes, nets, harness, budgets, conditions, analysis settings and catalog items. Quantities carry units, and every value records where it came from (catalog, override or kind default) and what backs it (datasheet, measurement, placeholder…). |
| **CLI and GUI as equals** | Every edit is an `Op`, and both front ends call the same service functions. A parity test checks that the same edit made from the CLI and from the HTTP API writes byte-identical files. |
| **GUI** | Budgets overview, part table, mode matrix (ON/OFF and duty cycles), electrical system diagram (power and signal, overlaid or separate), editable 3D layout, and one page per subsystem. Japanese and English UI. |
| **Orbit and environment** | Kepler/J2 and numerical propagation (DOP853), EGM96 gravity to degree 70, NRLMSISE-00, DE440 ephemerides (fetched on demand), central bodies other than Earth, solid tides, relativistic correction, ground track. |

## Analyses

| Subsystem | Analyses |
|---|---|
| Power | Mode power · sunlit generation · orbit generation · energy balance and depth of discharge |
| AOCS | RW unloading interval · MTQ/RCS unloading time · sensor field-of-view keep-out |
| C&DH | Data budget and storage |
| Communications | Link budget · ground-station visibility · downlink throughput |
| Thermal | Steady-state and transient multi-node |
| Structure | Mass properties · quasi-static loads · simple modal estimate |
| Propulsion | Maneuvers (impulsive · finite burn · low thrust) · delta-V budget with margin · propellant and tank margin · thruster usage against ratings |

Each analysis takes typed SI inputs built from the design, returns typed outputs with pass/warning/violation
checks, and lists any inputs that are still missing.

## Install

Each release attaches the `fune` binary with the GUI embedded:

| File | For |
|---|---|
| `fune-<version>-x86_64-linux-gnu.tar.gz` | x86_64 Linux with glibc 2.39 or later (Ubuntu 24.04 or newer, including WSL) |
| `fune-<version>-universal-apple-darwin.tar.gz` | macOS on Apple Silicon and Intel |

With [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), which downloads the release binary instead of
building it:

```bash
cargo binstall --git https://github.com/akira-nkjm/iwafune fune-cli
```

Or download the release by hand, with the GitHub CLI or from the Releases page:

```bash
gh release download --repo akira-nkjm/iwafune --pattern 'fune-*-x86_64-linux-gnu.tar.gz*'
sha256sum -c fune-*-x86_64-linux-gnu.tar.gz.sha256
tar -xzf fune-*-x86_64-linux-gnu.tar.gz
install -m 755 fune-*-x86_64-linux-gnu/fune ~/.local/bin/fune   # any directory on PATH
```

On macOS, use `universal-apple-darwin` in place of `x86_64-linux-gnu` and check the file with `shasum -a 256 -c`.
The binary is not notarized: downloaded with `gh` it runs as is, but a copy downloaded in a browser is blocked by
Gatekeeper until you run `xattr -d com.apple.quarantine fune`.

Then run `fune gui` in a design directory (one with `fune.toml`), or `fune init` to start one.
`fune data fetch de440` downloads the optional DE440 ephemeris the same way, and `fune data fetch space_weather` downloads the CelesTrak space weather table (F10.7, Ap) into the user cache. On other platforms, build from source.

### Updating

`fune self-update` replaces the running binary with the latest release. `fune self-update --check` only reports the current and latest versions, and `fune self-update --version 0.5.2` installs exactly that version (an older one too).
It downloads the archive with `curl`, verifies its SHA-256, checks that the new binary reports the expected version, and then swaps it in, so it needs write access to the directory holding `fune`.
With cargo-binstall, running the install command again also works. If `fune` came from another package manager, update it with that instead.

## Quick start

To build from source: Rust 1.100 or later (edition 2024), Node.js 24 or later, pnpm.

```bash
pnpm -C web install && pnpm -C web build                       # build the GUI (the fune binary serves web/dist)
cargo run -p fune-cli -- -C fixtures/demo-6u gui                # open the GUI on the demo 6U satellite
cargo run -p fune-cli -- -C fixtures/demo-6u part ls            # list parts from the CLI
cargo run -p fune-cli -- -C fixtures/demo-6u analyze power.energy_balance
```

`fune --help` lists every command. `fune analyze ls` lists the analyses.

## Repository layout

| Path | Contents |
|---|---|
| `crates/lib/units` (`sat-units`) | Units, dimensions and typed quantities |
| `crates/lib/astro` (`sat-astro`) | Math, integrators, time, orbits, ephemerides, eclipses, Earth models, attitude laws |
| `crates/lib/analysis` (`sat-analysis`) | Subsystem analyses with typed SI inputs and outputs; no knowledge of the design model |
| `crates/app/model` (`fune-model`) | Design data types, built-in vocabulary, resolution and validation |
| `crates/app/store` (`fune-store`) | Reading and writing the TOML directory, revisions, locking |
| `crates/app/ops` (`fune-ops`) | `Op`, the single entry point for edits: plan, apply, and conversion to CLI commands |
| `crates/app/projection` (`fune-projection`) | Resolved design → analysis inputs, with a list of missing inputs |
| `crates/app/service` (`fune-service`) | Every function the CLI and the HTTP API call, plus their types (exported to TypeScript) |
| `crates/app/server` (`fune-server`) | HTTP API (axum) and the embedded GUI |
| `crates/app/cli` (`fune-cli`) | The `fune` command |
| `web/` | GUI (React, TypeScript) |
| `fixtures/demo-6u` | Demo 6U satellite |
| `docs/` | Architecture overview, data model, architecture registry, roadmap |

`crates/lib/*` never depend on `crates/app/*`. `cargo xtask check` enforces the dependency direction and the
"one capability, one place" rules recorded in [docs/architecture/registry.md](docs/architecture/registry.md).

## Development

```bash
cargo xtask check                              # dependency and registry rules
cargo test --workspace
cargo clippy --all-targets -- -D warnings
pnpm -C web lint && pnpm -C web typecheck && pnpm -C web test
cargo test -p fune-service export_ts           # regenerate web/src/api/gen after changing API types
```

CI runs all of the above, then builds the release binary and smoke-tests the CLI and the HTTP server.

### Branches and releases

Daily work goes to `develop`. `main` only receives pull requests from `develop`, and each one is a release: bump
the version on `develop` first (`[workspace.package] version` in `Cargo.toml` and `version` in `web/package.json`,
to the same value). [release.yml](.github/workflows/release.yml) checks on the pull request that the version is
consistent and not released yet; after the merge it publishes the container image, then creates the tag `vX.Y.Z`
and its GitHub Release. Tags are not pushed by hand.

### fune in other repositories' CI

Each release publishes the CLI as `ghcr.io/akira-nkjm/fune:<version>` (and `:latest`) through
[image.yml](.github/workflows/image.yml). To let another repository pull it, add that repository under the
package's "Manage Actions access"; its workflow then uses the image as a job container with its own
`GITHUB_TOKEN` (`permissions: packages: read`) and runs `fune check`.

### Shared catalogs in CI

A satellite repository can read catalog items from a shared catalog repository (`[[catalog]] git = …` in
`fune.toml`, see [docs/model.md §12](docs/model.md)). The commit is pinned in `catalog.lock`, which is committed,
and the fetched files are cached under `.fune/` (ignored by git). Add one with
`fune catalog source add <name> --git <url> [--rev <rev>]` (or from the catalog page of the GUI), then run
`fune catalog fetch` and commit `fune.toml` and `catalog.lock`; `fune catalog source rm <name>` removes it again
(refused while parts use its items). In CI, fetch the locked commit first, then check:

```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    container: ghcr.io/akira-nkjm/fune:latest
    permissions:
      contents: read
      packages: read
    steps:
      - uses: actions/checkout@v4
      # Only for a private catalog repository: let git authenticate with a token that can read it.
      - run: git config --global url."https://x-access-token:${{ secrets.CATALOG_TOKEN }}@github.com/".insteadOf "https://github.com/"
      - run: fune catalog fetch --locked   # fails if catalog.lock is missing or does not match fune.toml
      - run: fune check
```

`fune catalog fetch --locked` never rewrites `catalog.lock`; run `fune catalog fetch` (or `fune catalog update <name>`
to move to the newest commit of `rev`) locally and commit the lock. fune calls the `git` command, so authentication
is whatever your git configuration provides (credential helper, SSH keys); the image includes `git`.

## Documentation

The design documents are written in Japanese.

- Architecture overview: [docs/architecture.md](docs/architecture.md)
- Design model and file format: [docs/model.md](docs/model.md)
- Architecture registry (where each capability lives): [docs/architecture/registry.md](docs/architecture/registry.md)
- Roadmap toward conceptual design: [docs/roadmap.md](docs/roadmap.md) (tracked in issues #1–#16)
- Development rules for contributors: [CONTRIBUTING.md](CONTRIBUTING.md)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Parts derived from other works, and the fonts bundled with the GUI, keep their own terms; see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
