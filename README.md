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
  <b>Design data in git</b> · <b>CLI and GUI as equals</b> · <b>27 analyses across 8 areas</b>
</p>

**Documentation: <https://akira-nkjm.github.io/iwafune/>**

iwafune is a workbench for the conceptual design of small satellites. The design lives in a git-friendly
directory of TOML files. You edit it from either the `fune` CLI or a web GUI, and run subsystem analyses
on the same data.

## What you can do

- Pick parts from a catalog, set values with units, and record where each value comes from (datasheet, measurement, placeholder).
- Define operating modes and see which part is on, off or duty-cycled in each.
- Track mass, power and data budgets against allocations and margins.
- Draw the electrical system (power and signal) and place parts in an editable 3D layout.
- Run subsystem analyses, and read pass/warning/violation verdicts and the inputs still missing.
- Save analysis settings as cases and compare them.
- Use the CLI and the GUI interchangeably: both make the same edits, and the result is plain files in git.

The GUI has Japanese and English UI. The CLI, docs and demo data are in English.

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

Then see [Getting started](#getting-started). On other platforms, [build from source](docs/development.md#building-from-source).

### Updating

`fune self-update` replaces the running binary with the latest release. `fune self-update --check` only reports the current and latest versions, and `fune self-update --version 0.5.2` installs exactly that version (an older one too).
It downloads the archive with `curl`, verifies its SHA-256, checks that the new binary reports the expected version, and then swaps it in, so it needs write access to the directory holding `fune`.
With cargo-binstall, running the install command again also works. If `fune` came from another package manager, update it with that instead.

## Getting started

### Get a project

Start a new one with `fune init <DIR>`. It creates the directory (parents included) and the project inside it; the project name is the last part of the path, or `--name`. It does not run `git init`:

```bash
fune init mysat
cd mysat
git init
fune status
```

`fune init` with no directory creates the project in the current directory (or in `-C <dir>`), named after that directory; add `--name` to choose another name. An existing directory is fine as long as it has no `fune.toml`. (Before this change `fune init <NAME>` wrote into the current directory; use `fune init --name <NAME>` for that.)

Or explore the demo 6U satellite. It is in the repository (there is no template option in `fune init`); copy it so you can edit freely:

```bash
git clone https://github.com/akira-nkjm/iwafune
cp -r iwafune/fixtures/demo-6u my-demo && cd my-demo
fune status
```

`fune` finds the project by searching upward for `fune.toml`; `-C <dir>` points it elsewhere.

### Open the GUI

```bash
fune gui            # serves on http://127.0.0.1:3000 (the next free port if busy) and opens the browser
fune gui --no-open  # do not open the browser
```

The sidebar has Overview, Design (Components, Catalog, System diagram, Modes, Layout, Maneuvers), one page per subsystem,
Conditions (Orbit & stations, Analysis cases) and Project (Units & definitions, Diagnostics, Settings, Sources).
Pages show the CLI command for an edit, so you can learn one from the other. The CLI examples below use the demo.

### Walk through

| Step | CLI | In the GUI |
|---|---|---|
| Look around | `fune part ls`, `fune part show RW_X`, `fune catalog ls` | Components, Catalog |
| Add a part from a catalog item | `fune part add FSS_2 --catalog local/fss_60 --group aocs` | Components (add) |
| Set a value with a unit | `fune set FSS_2.mass "45 g" --source <id>` (`fune get FSS_2.mass` reads it back) | Components, the part's page |
| Put the part in a mode | `fune mode set NOMINAL FSS_2 on` (`fune mode table` shows all modes) | Modes |
| Group parts | `fune part mv FSS_2 aocs/sensors` (creates the group) | Components |
| Check the design | `fune check` | Diagnostics (and the counter in the header) |
| Read the budgets | `fune budget mass`, `fune budget power`, `fune budget data` | Overview |
| Run an analysis | `fune analyze power.energy_balance` | the subsystem page, e.g. Power |
| Run it for a case | `fune analyze power.energy_balance --case hot_imaging` | the subsystem page, case selector; Analysis cases |
| Define a case | `fune case set my_case --mode NOMINAL --label "My case"` | Analysis cases |
| Undo a mistake | `fune history`, `fune undo`, `fune redo` | Change history in the header |
| Save | `git add -A && git commit -m "Add FSS_2"` | (use git) |

Add `--dry-run` to an edit to see the file diff without writing, and `--json` to any command for machine-readable output.

Values written without a source are accepted but marked "Unsourced". Register a source first with
`fune source set <id> --kind datasheet --label "..."` and pass it with `--source <id>`. The kinds are placeholder, judgement, analysis, datasheet and measured.

### Analyses, verdicts and missing inputs

An analysis builds its inputs from the design and returns KPIs and verdicts (ok, warning, violation).
If the design lacks something, it stops and lists what to provide, for example on a fresh project:

```console
$ fune analyze comm.link_budget
✖ comm.link_budget: missing inputs (3)
 Missing input       Reason
 analysis.comm       Comm analysis settings (analysis/comm.toml)
 conditions.ground   Ground station (conditions/ground/<id>.toml)
 conditions.orbit    Orbit settings (conditions/orbit.toml)
```

Fill these in from the GUI (Orbit & stations, the subsystem page) or with `fune setting` / `fune apply`; the file shapes are in [docs/model.md](docs/model.md).
`fune check` also warns about required attributes that are not set (for example a reaction wheel without `max_torque`) and budget limits that are not set (a budget limit is an `at_most` requirement: `fune requirement add R_MASS --statement "..." --quantity Mass --kind at_most --value "3 kg" --budget mass`).

## Everyday commands

| Command | What it does |
|---|---|
| `fune status` | Project overview: counts of parts, modes, nets, cases |
| `fune check` | Validate the design; exit code 1 on errors |
| `fune part ls` / `fune part show <id>` | List parts / everything about one part |
| `fune part add <id> --catalog <item>` | Add a part (or `--kind <kind>` for a blank one) |
| `fune set <part>.<attr> "<value unit>"` | Set a value; `fune get` reads it, `fune unset` reverts to the catalog value |
| `fune mode table` | Which part is in which state in each mode |
| `fune budget mass\|power\|data` | Budget with growth, allocation and margin |
| `fune analyze ls` / `fune analyze <id>` | List analyses / run one (`--case <id>` for a case) |
| `fune catalog add` / `fune catalog show <item>` | Add or inspect catalog items |
| `fune fmt` / `fune fmt --check` | Rewrite files in canonical form / check them |
| `fune undo` / `fune redo` | Step through the last 100 edits |
| `fune gui` | Start the GUI |
| `fune self-update` | Update the binary |
| `fune skill show` / `fune skill install` | Print / install the agent skill (see below) |

`fune --help` lists every command, and `fune <command> --help` shows its options.

### Agent skill

`fune skill install` writes an Agent Skills file, `.claude/skills/fune/SKILL.md`, into the project (the directory with `fune.toml`, or the current directory outside a project),
so an AI agent knows the rules for editing and analyzing a design: edit only through the CLI, run `fune check` and `fune fmt --check`, write units, and the usual flows.
`--skills-dir <PATH>` installs into another skills directory (for another agent's location), and `fune skill show` prints the text.
The text ships inside the binary and records the `fune` version. Running `fune skill install` again does nothing if it is current, replaces it if another version wrote it,
and refuses (exit code 3) if you edited it, unless you pass `--force`.

## Analyses

`fune analyze ls` lists them (27 on this version). Each takes typed SI inputs built from the design.

| Area | Analyses |
|---|---|
| AOCS | RW unloading frequency, momentum unloading time, sensor FoV and exclusion angles, slew time |
| C&DH | Data volume and storage |
| Communications | Link budget, ground station visibility, downlink capacity and backlog |
| Orbit | Beta angle and eclipse seasons, drag decay and station keeping |
| Power | Mode power consumption, sunlit generation, orbital generation, energy balance and DoD, power sizing, energy balance while thrusting |
| Propulsion | Maneuvers, delta-V budget, propellant and tanks, thruster usage, orbit history, mass properties with propellant |
| Structure | Mass properties, quasi-static loads, natural frequency estimate |
| Thermal | Steady state, transient |

Orbit modelling includes Kepler/J2 and numerical propagation, EGM96 gravity, NRLMSISE-00, DE440 ephemerides, central bodies other than Earth and ground tracks.

## Working in a team

The design is a directory of TOML files, so branches, pull requests and reviews work as they do for code.
`fune fmt` keeps the files in canonical form so diffs stay small; run `fune fmt --check` and `fune check` before committing.

### Checking in CI

Each release publishes the CLI as the container image `ghcr.io/akira-nkjm/fune:<version>` (and `:latest`). To let another repository pull it,
add that repository under the package's "Manage Actions access". The workflow then uses it as a job container with its own `GITHUB_TOKEN`:

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
      - run: fune fmt --check
      - run: fune check
```

### Shared catalogs

A satellite repository can read catalog items from a shared catalog repository (see [docs/model.md §12](docs/model.md#12-shared-catalogs)). Items appear as `<name>/<ID>`.

```bash
fune catalog source add shared --git <url> [--rev <rev>]   # or from the Catalog page of the GUI
fune catalog fetch                                          # fetch and write catalog.lock
git add fune.toml catalog.lock && git commit -m "Add shared catalog"
fune catalog update shared    # later: move to the newest commit of rev and show what changed
fune catalog source rm shared # refused while parts use its items
```

The commit is pinned in `catalog.lock` (commit it); fetched files are cached under `.fune/` (ignored by git). In CI, fetch the locked commit before checking:

```yaml
      - uses: actions/checkout@v4
      # Only for a private catalog repository: let git authenticate with a token that can read it.
      - run: git config --global url."https://x-access-token:${{ secrets.CATALOG_TOKEN }}@github.com/".insteadOf "https://github.com/"
      - run: fune catalog fetch --locked   # fails if catalog.lock is missing or does not match fune.toml
      - run: fune check
```

`fune catalog fetch --locked` never rewrites `catalog.lock`. fune calls the `git` command, so authentication is whatever your git configuration provides; the image includes `git`.

## Optional data

Some analyses use data that is not bundled. `fune data status` shows what is cached.

```bash
fune data fetch de440           # DE440 ephemeris (needs the GitHub CLI, gh)
fune data fetch space_weather   # CelesTrak space weather table, F10.7 and Ap (needs curl)
fune data fetch radiation       # AP-8/AE-8 trapped-radiation tables and geomagnetic coefficients (needs gh;
                                # or `--from <DIR>` to install from a directory that holds the files)
```

All of them go to the user cache. The sources and attribution of the radiation data are in [docs/trapped-radiation.md](docs/trapped-radiation.md).

## Learn more

- [docs/model.md](docs/model.md): design data, file formats, directory layout
- [docs/architecture.md](docs/architecture.md): how the code is put together
- [docs/roadmap.md](docs/roadmap.md): planned features (tracked in GitHub Issues)
- [docs/development.md](docs/development.md): building from source, repository layout, releases
- [CONTRIBUTING.md](CONTRIBUTING.md): rules for contributors

The documents are written in English.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Parts derived from other works, and the fonts bundled with the GUI, keep their own terms; see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
