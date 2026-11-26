# Install

This page shows how to install `fune`, check that it runs, update it, fetch the optional data sets and remove it. `fune` is a single program: it holds the command-line tool and the web GUI, so there is nothing else to install.

## Choose a method

| Method | Use it when |
|---|---|
| [Release binary](#release-binary) | You want the simplest install. Linux (x86_64) and macOS. |
| [cargo-binstall](#cargo-binstall) | You already use Rust tools and want one command. |
| [Container image](#container-image) | You only want to run `fune check` in a CI job. |
| [From source](#from-source) | Your platform has no release binary, or you work on iwafune itself. |

Every release is published on the GitHub Releases page of the repository. Each release attaches these files:

| File | For |
|---|---|
| `fune-<version>-x86_64-linux-gnu.tar.gz` | x86_64 Linux with glibc 2.39 or later (Ubuntu 24.04 or newer, including WSL) |
| `fune-<version>-universal-apple-darwin.tar.gz` | macOS on Apple Silicon and Intel |

Each archive has a matching `.sha256` file.

## Release binary

Download the archive with the GitHub CLI (`gh`) or from the Releases page, check the checksum, unpack it, and put `fune` in a directory on your `PATH`. These commands are for Linux:

```bash
gh release download --repo akira-nkjm/iwafune --pattern 'fune-*-x86_64-linux-gnu.tar.gz*'
sha256sum -c fune-*-x86_64-linux-gnu.tar.gz.sha256
tar -xzf fune-*-x86_64-linux-gnu.tar.gz
install -m 755 fune-*-x86_64-linux-gnu/fune ~/.local/bin/fune
```

On macOS, use `universal-apple-darwin` in place of `x86_64-linux-gnu`, and check the file with `shasum -a 256 -c`.

The macOS binary is not notarized. A file downloaded with `gh` runs as it is. A file downloaded in a browser is blocked by Gatekeeper until you remove the quarantine flag:

```bash
xattr -d com.apple.quarantine fune
```

Then go to [Check the install](#check-the-install).

## cargo-binstall

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) downloads the release binary instead of building it:

```bash
cargo binstall --git https://github.com/akira-nkjm/iwafune fune-cli
```

Then go to [Check the install](#check-the-install). To update later, run `fune self-update` (see [Update](#update)).

## Container image

Each release also publishes the CLI as a container image, `ghcr.io/akira-nkjm/fune:<version>` (and `:latest`). The image includes `git`. It is meant for checking a design in CI.

To let another repository pull the image, add that repository under the package's "Manage Actions access". The workflow then uses the image as a job container with its own `GITHUB_TOKEN`:

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

The [CLI guide](cli.md#shared-catalogs) shows the extra step for shared catalogs.

## From source

Use this when no release binary fits your platform. You need Rust 1.100 or later, Node.js 24 or later, and pnpm. The GUI is built first, because the `fune` binary serves the built files:

```bash
pnpm -C web install && pnpm -C web build
cargo run -p fune-cli -- -C fixtures/demo-6u part ls
```

The second command builds `fune` and lists the parts of the demo project. You should see a table of parts. The full developer workflow, the repository layout and the release process are in [Development](../development.md).

## Check the install

```console
$ fune --version
fune 0.10.0
$ fune --help
The `fune` command-line interface: argument parsing and output only (tables, colors, diagnostics)

Usage: fune [OPTIONS] <COMMAND>

Commands:
  init           Create a new project
  check          Check the design data (exit code 1 on errors)
  gui            Start the GUI (a local HTTP server) and open it in the browser
  ...
```

You should see a version line and the list of commands. Your version number will differ. If the shell says `command not found`, the directory that holds `fune` is not on your `PATH`.

To try the program on a design, follow the [quickstart](quickstart.md).

## Update

`fune self-update` replaces the running binary with the latest release. First look at what it would do:

```console
$ fune self-update --check
Checking the latest release...
✔ fune 0.10.0 is already the latest version
```

If a newer release exists, the output says so. Run `fune self-update` to update. To install an exact version, even an older one, use `fune self-update --version <major.minor.patch>`.

Before it replaces anything, `self-update` downloads the archive with `curl`, unpacks it with `tar`, verifies its SHA-256 and checks that the new binary reports the expected version. If any step fails, it replaces nothing. It needs write access to the directory that holds `fune`. This works whichever way the binary was installed (release archive, cargo-binstall or a build from source).

`fune self-update` changes the program itself. It does not touch your projects.

## Optional data sets

A few analyses use data that is too large to bundle. Until you fetch the data, they report it as a missing input and do not run. Check what is cached:

```console
$ fune data status
 ID                 State               Size      Loaded   Location
─────────────────────────────────────────────────────────────────────────────────────────────
 de440              ✔ SHA-256 matches   20.7 MB   yes      ~/.cache/iwafune/data/de440.bin
 ap8min             ✔ SHA-256 matches    102 kB   yes      ~/.cache/iwafune/data/ap8min.asc
 ap8max             ✔ SHA-256 matches    101 kB   yes      ~/.cache/iwafune/data/ap8max.asc
 ae8min             ✔ SHA-256 matches   81.3 kB   yes      ~/.cache/iwafune/data/ae8min.asc
 ae8max             ✔ SHA-256 matches   83.6 kB   yes      ~/.cache/iwafune/data/ae8max.asc
 jensen_cain_1960   ✔ SHA-256 matches   2.27 kB   yes      ~/.cache/iwafune/data/jensen_cain_62.dat
 gsfc_1966          ✔ SHA-256 matches   36.6 kB   yes      ~/.cache/iwafune/data/gsfc66.dat
 igrf14             ✔ SHA-256 matches   42.4 kB   yes      ~/.cache/iwafune/data/igrf14coeffs.txt
 shieldose2         ✔ SHA-256 matches    712 kB   yes      ~/.cache/iwafune/data/shieldose2.dat
 space_weather      not fetched               —   —        ~/.cache/iwafune/data/SW-All.csv
Fetch: fune data fetch space_weather
```

The table above is from a machine where the radiation data and the ephemeris were already installed. On a fresh install most rows say `not fetched`. The `Location` column is shortened to `~/…` here. On your machine it shows the real cache path.

What each data set is for:

| ID | Used by |
|---|---|
| `de440` | Positions of the Sun, Moon and planets. Without it, analytic expressions are used, and the dated transfers of the propulsion analyses use it. |
| `space_weather` | Solar flux F10.7 and geomagnetic Ap for the atmosphere model (drag decay by date). |
| `ap8min`, `ap8max`, `ae8min`, `ae8max`, `jensen_cain_1960`, `gsfc_1966`, `igrf14`, `shieldose2` | Trapped-radiation analyses: `orbit.trapped_flux`, `orbit.trapped_dose`, and the computed environment of `orbit.total_dose`. One command installs the whole group. |

Fetch the data you need. These commands download from the internet and write to the user cache, not to your project:

```bash
fune data fetch de440           # needs the GitHub CLI, gh
fune data fetch space_weather   # needs curl
fune data fetch radiation       # needs gh
```

If you already have the radiation files in a directory, install them from there instead:

```bash
fune data fetch radiation --from <DIR>
```

`fune data fetch` checks size and SHA-256, with `--from` too. After a fetch, `fune data status` shows `✔ SHA-256 matches` for that row. The sources and attribution of the radiation data are in [Trapped radiation data](../trapped-radiation.md). You can also fetch the data from the GUI: see [Settings](gui.md#settings).

## Uninstall

Remove the binary, and the data cache if you no longer want it:

```bash
which fune                 # shows where the binary is
rm "$(which fune)"
rm -r ~/.cache/iwafune     # the data cache; the Location column of `fune data status` shows its path
```

Your projects are ordinary directories. Nothing outside them needs cleaning. If you used the agent skill (`fune skill install`), delete `.claude/skills/fune/` in the project too.
