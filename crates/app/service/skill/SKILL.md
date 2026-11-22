---
name: fune
description: Edit and analyze satellite conceptual-design data with the `fune` CLI (a git-managed directory of TOML with a fune.toml at its root). Use it whenever you add or change parts, modes, settings, requirements or analysis cases in such a project, run subsystem analyses or budgets and read their verdicts, check requirement compliance, or export report tables. Do not hand-edit the design files.
metadata:
  fune-version: "{{FUNE_VERSION}}"
---

# Working on a fune design

A fune project is a directory with `fune.toml`. Its design data (`parts/`, `modes/`, `catalog/`,
`cases/`, `requirements/`, ...) is TOML that the `fune` CLI reads, edits and analyzes.
This text is the rules and the usual flows. For the exact arguments of any command, run
`fune --help` and `fune <command> --help`; do not guess flags.

## Rules

1. **Never edit the design TOML by hand.** Every edit goes through the CLI (or `fune apply` with Op
   JSON). The CLI keeps references, the canonical file format and the edit history consistent.
   Reading the files is fine.
2. **After every edit run both checks** and fix what they report before going on:
   `fune check` (exit code 1 on errors; `--deny-warnings` treats warnings as errors) and
   `fune fmt --check` (exit code 1 if a file is not in canonical form; `fune fmt` rewrites them).
3. **Values carry units**, written in quotes: `"620 g"`, `"6.8 W"`, `"-20 .. 60 degC"`. A bare number
   is rejected for a quantity. `fune units parse "18 mN*m*s"` shows how a string is read, and
   `fune units ls` lists the units.
4. **Look before you write.** `fune status`, `fune part ls`, `fune catalog ls`, `fune mode ls`
   show what exists, so you reuse IDs instead of inventing them.
5. **Try an edit first** with `--dry-run` (prints the diff, writes nothing). Every editing command
   has it. A mistake can be reverted with `fune undo` (`fune history` lists the edits).
6. **Say where a number comes from.** Attach `--source <id>` (a source registered in `sources/`,
   see `fune source ls`) when a value comes from a datasheet or a test.
7. Run the CLI from the project directory, or pass `-C <dir>`. Use `--json` when a program (you)
   reads the output.

## IDs and paths

- Part IDs are file names under `parts/`, for example `RW_X`. A group is a directory:
  `fune part mv RW_X aocs/RW` moves a part into the group `RW` under the subsystem `aocs`.
  Subsystem directories are `aocs`, `cdh`, `comm`, `payload`, `power`, `propulsion`, `structure`, `thermal`.
- A value is addressed as `<part>.<attribute>` (`RW_X.mass`), with a state for state-dependent
  values (`RW_X.power.on`). Settings use `analysis.power.dod_max`. Catalog items are
  `local/<name>` (or `<catalog>/<name>`), for example `local/bat_2s2p`.
- Modes (`NOMINAL`, `SAFE`, ...), analysis cases and requirements are referred to by their IDs.
- Analyses are `<system>.<name>`, for example `power.energy_balance`. `fune analyze ls` lists them.
- Do not split IDs or paths yourself; ask the CLI (`fune part show <id>`, `--json`).

## Typical flows

Orient yourself:

```
fune status
fune part ls
fune part show RW_X
fune mode table
```

Add a part from a catalog item and describe it:

```
fune part add TRX2 --catalog local/uhf_trx_9k6 --group comm --dry-run
fune part add TRX2 --catalog local/uhf_trx_9k6 --group comm
fune part place TRX2 --on BUS_FRAME --position "0 0 12 mm"
fune set TRX2.mass "95 g" --source em_test_2026_10
fune mode set NOMINAL TRX2 rx
fune check
fune fmt --check
```

(`fune part add` also takes `--kind <kind>` for a blank part. A new part gets a row in every mode,
default state `off`; set the states it really has with `fune mode set`.)

Modes: `fune mode add SURVEY --from NOMINAL` copies a table, then adjust rows with
`fune mode set <mode> <part> <state>` and `fune mode duration <mode> --max-duration "90 min"`.

Run analyses and read the verdicts:

```
fune analyze ls
fune analyze power.energy_balance
fune analyze power.energy_balance --case eclipse_worst
fune budget mass
```

`fune case ls` lists the analysis cases. A verdict is OK, a warning or a violation. `fune analyze`
exits with 1 if an input is missing, the run fails or a check is a violation. A missing input is
fixed by setting the value it names (`fune set ...`), not by changing the analysis.
`--ops <file.json>` evaluates unsaved edits without writing them.

Requirements:

```
fune requirement ls
fune requirement compliance
fune requirement add R_MASS --statement "The satellite total mass shall not exceed 12 kg." --quantity Mass --kind at_most --value "12 kg" --budget mass
```

`fune requirement compliance` exits with 1 on a violation; "not verified" means no analysis check
or budget is linked to that requirement yet. Check `fune requirement add --help` for the exact link flags.

Report tables and figures for a Typst document:

```
fune report export --out exports/report --case eclipse_worst
```

## Exit codes

0 success; 1 the design or an analysis has errors or violations; 2 bad arguments or something not
found; 3 conflict (the design changed since `--base-revision`, or the same ID already exists).
With `--json`, errors are printed as `{"error": {"code", "message"}, "diagnostics": [...]}`.

## More

- `fune --help` lists every command; `fune <command> --help` shows its flags.
- `fune skill show` prints this text. Re-run `fune skill install` after updating `fune` so the
  text matches the binary (this text is for fune {{FUNE_VERSION}}).
- If the repository has `docs/model.md`, it describes the data model and the file formats.
