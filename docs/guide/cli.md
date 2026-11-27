# CLI guide

The `fune` command reads and edits a design project (a git-managed directory of TOML files), runs analyses and budgets, and prints the result. This page is organised by task. For each task you get the commands, a real example on the demo satellite, and what the command changes. A table of every command is at the end.

The examples use a copy of the demo project (`fune init my-demo --demo`), as in the [quickstart](quickstart.md). Most edits are shown with `--dry-run`, which prints the diff and writes nothing, so you can try them without changing your copy. Output is shortened with `…` where it is long. Paths are shown relative to the project.

`fune --help` lists the commands, and `fune <command> --help` shows the options of one. When this page and the help text differ, the help text is right.

Contents:

- [Global options](#global-options)
- [Create and inspect a project](#create-and-inspect-a-project)
- [Read and write values](#read-and-write-values)
- [Parts and catalog](#parts-and-catalog)
- [Modes and sequences](#modes-and-sequences)
- [Cases, phases and configurations](#cases-phases-and-configurations)
- [Requirements](#requirements)
- [Links and ground stations](#links-and-ground-stations)
- [Maneuvers](#maneuvers)
- [Nets, harnesses, definitions and diagram layout](#nets-harnesses-definitions-and-diagram-layout)
- [Run analyses and budgets](#run-analyses-and-budgets)
- [Apply Op JSON](#apply-op-json)
- [Check, format, undo](#check-format-undo)
- [Export the design document](#export-the-design-document)
- [Data sets](#data-sets)
- [Shared catalogs](#shared-catalogs)
- [Agent skill, updates and the GUI](#agent-skill-updates-and-the-gui)
- [Command reference](#command-reference)

## Global options

These options work on every command:

| Option | Meaning |
|---|---|
| `-C, --dir <DIR>` | The project: a directory that has `fune.toml`, or one below it. Default: the current directory. `fune` searches upward for `fune.toml` |
| `--json` | Print machine-readable JSON instead of tables |
| `--color <auto\|always\|never>` | Colour. `auto` is plain when output is not a terminal or `NO_COLOR` is set |
| `-h, --help` | Help for the command |
| `-V, --version` | Version of `fune` |

Commands that write to the design share two more options:

| Option | Meaning |
|---|---|
| `--dry-run` | Show the diff of the files that would change, and write nothing |
| `--base-revision <REV>` | Do not write if the design changed since this revision (exit code 3). For scripts that must not overwrite another edit |

A few commands take neither: `fune fmt` and `fune undo` / `fune redo` change files but are not edits, and the commands that only read have nothing to write.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | The design or the result fails: a check error, a missing input, a failed run, a violation verdict |
| 2 | Invalid input, or not found |
| 3 | Conflict: the design changed under you, or the ID already exists |

Warnings never fail a command, except `fune check --deny-warnings`.

### JSON output

`--json` works with every command. For example, the counts of a project:

```console
$ fune status --json
{
  "name": "iwafune-6u-demo",
  "root": "my-demo/.",
  "revision": "b3:5e54b14f210acafe01e4f5c5e43246567e0e32e81bd8f04f1978c00f7365bafd",
  "branch": "main",
  "schema": 2,
  "counts": {
    "parts": 38,
    "groups": 11,
    "modes": 4,
    "nets": 8,
    "harnesses": 2,
    "cases": 3
  },
  …
}
```

A write with `--dry-run --json` also returns the equivalent command in a `cli` array, which is useful when you want to learn the command for a GUI action.

## Create and inspect a project

**Create a project.** `fune init [DIR]` creates the directory and a minimal project inside it, with `fune.toml`, budget files, one mode and a `.gitignore`. It does not run `git init`. The project name is the last part of the path; `--name` sets it. It refuses a directory that already has a `fune.toml`.

```console
$ fune init my-sat
✔ Created project my-sat: my-sat/…
  fune.toml
  budget/allocation.toml
  budget/policy.toml
  modes/SAFE.toml
  .gitignore
Next:
  cd my-sat
  fune status
```

**Create the demo.** `fune init [DIR] --demo` creates the demo 6U satellite that is bundled into `fune`, instead of an empty project. It keeps its own project name, so `--name` cannot be combined with it, and it refuses to write if `fune.toml` or any file it would write already exists.

```console
$ fune init my-demo --demo
✔ Created project iwafune-6u-demo: my-demo/…
  .gitignore
  analysis/aocs.toml
…
Next:
  cd my-demo
  fune status
  fune gui
```

**Overview and checks.** `fune status` counts what the project has and summarises the diagnostics. `fune check` validates the design:

```console
$ fune status
iwafune-6u-demo
my-demo/.  branch main  rev b3:5e54b14f2
parts 38  groups 11  modes 4  nets 8  harnesses 2  cases 3
✔ Check: no problems
```

**List and show.** Most nouns have `ls`:

```console
$ fune part ls
 ID           Subsystem    Kind                  Definition          Count   Mass     Power         Operating temp
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 BAT          power        battery               local/bat_2s2p          1    395 g             —   0 °C … 45.0 °C
 BUS_FRAME    structure    structure             —                       1    620 g             —   not set
 CAM          payload      camera                local/cam_ms_5m         1    920 g   6.80 W (on)   0 °C … 35.0 °C
 …
```

`fune part show <ID>` prints everything about one part, grouped by subsystem: values and their sources, placement and ports. `fune catalog show <ITEM>`, `fune phase show <ID>` and `fune configuration show <ID>` do the same for their things. Other lists: `fune mode ls`, `fune net ls`, `fune harness ls`, `fune case ls`, `fune source ls`, `fune requirement ls`, `fune maneuver ls`, `fune kind ls`, `fune attr ls`, `fune units ls` and `fune setting ls`.

## Read and write values

`fune get` reads a value and says where it comes from. `fune set` writes it. `fune unset` removes an override.

```console
$ fune get RW_X.mass
152 g  Override, em_test_2026_10, Measured
$ fune get RW_X.power --state on
600 mW  Catalog, Datasheet
```

The path of a value is `<part>.<attribute>`, or `<part>.<attribute>.<state>` for a value by state. Without `--state`, `fune get` of a value by state shows every state: `off 0 W / on 650 mW  Catalog, Datasheet`.

`fune set` changes the value, and a source and a note if you give them:

```console
$ fune set RW_X.mass "160 g" --source em_test_2026_10 --note "Flight unit" --dry-run
--- a/parts/aocs/RW/RW_X.toml
+++ b/parts/aocs/RW/RW_X.toml
@@ -5,10 +5,10 @@
 point = "RW_SLOT_X"

 [attr.mass]
-value       = 0.152
-unit        = "kg"
+value       = 160.0
+unit        = "g"
 from.source = "em_test_2026_10"
-from.note   = "EM measurement"
+from.note   = "Flight unit"

 [ports.PWR]
 net = "P12V"
✔ dry-run: 1 files would change (nothing written)
```

The unit you write (`g`) is kept in the file. Files written by `fune` are always in canonical form, so `fune fmt --check` passes right after an edit.

| Option | Meaning |
|---|---|
| `--source <ID>` | The source of the value (an ID in `sources/`). It must exist |
| `--at <TEXT>` | Place in the document, for example `"p.12 table 3"` |
| `--note <TEXT>` | A free note |
| `--min <VALUE>`, `--max <VALUE>` | Lower and upper bound of the uncertainty, in the unit of the value (a different unit is refused) |

**What `set` does to an existing value.** `fune set` replaces the whole record of the value. If the value had a source and a note, and you do not pass them again, they are dropped:

```console
$ fune set RW_X.mass "160 g" --dry-run
--- a/parts/aocs/RW/RW_X.toml
+++ b/parts/aocs/RW/RW_X.toml
@@ -4,11 +4,8 @@
 on    = "PANEL_MZ"
 point = "RW_SLOT_X"

-[attr.mass]
-value       = 0.152
-unit        = "kg"
-from.source = "em_test_2026_10"
-from.note   = "EM measurement"
+[attr]
+mass = { value = 160.0, unit = "g" }

 [ports.PWR]
 net = "P12V"
✔ dry-run: 1 files would change (nothing written)
```

Pass `--source` again to keep the trail. `fune unset` removes the override and brings back the catalog value:

```console
$ fune unset RW_X.mass --dry-run
--- a/parts/aocs/RW/RW_X.toml
+++ b/parts/aocs/RW/RW_X.toml
@@ -4,12 +4,6 @@
 on    = "PANEL_MZ"
 point = "RW_SLOT_X"

-[attr.mass]
-value       = 0.152
-unit        = "kg"
-from.source = "em_test_2026_10"
-from.note   = "EM measurement"
-
 [ports.PWR]
 net = "P12V"

✔ dry-run: 1 files would change (nothing written)
```

### Value syntax

You type the value as text, and `fune` reads it against the definition of the attribute or the setting. You never say the type. The same reading is used by the GUI input fields, so a value that works here works there.

| Type of the value | You write | Example |
|---|---|---|
| Quantity | A number and a unit. The unit must have the dimension of the attribute | `"620 g"`, `"550 km"`, `"7.4 V"`, `"-20 degC"` |
| Number without a unit (a fraction, an efficiency) | A plain number | `0.28` |
| Range | `min .. max` and one unit at the end. Both numbers may carry the unit, but it must be the same | `"-10 .. 50 degC"` |
| Vector of three | Three numbers separated by spaces or commas, with an optional unit at the end | `"0 0 1"`, `"100 100 20 mm"` |
| Quaternion | Four numbers: w x y z | `"1 0 0 0"` |
| Curve | `x:y` pairs, then the units in brackets: the unit of x, `->`, the unit of y | `"0:480 10:360 20:150 (km^2/s^2 -> kg)"` |
| Boolean | `true` or `false` | `true` |
| Enumeration, text | The word or the text, as it is | `sun_pointing`, `"Wheel X"` |
| List setting | Comma-separated words, or a TOML array of strings | `"BAT_A, BAT_B"`, `'["BAT_A", "BAT_B"]'` |
| Time of day | `HH:MM` | `"10:30"` |

A number typed for a real-valued setting is stored as a real number: `fune set conditions.orbit.eccentricity 0` writes `eccentricity = 0.0`. A setting that is an integer, such as `cases.<ID>.orbits`, takes an integer.

`--dry-run` shows how a value is stored, which is the quickest way to learn the form of a range or a vector:

```console
$ fune set RW_X.temp_op "-10 .. 50 degC" --dry-run
…
+[attr]
+temp_op = { min = -10.0, max = 50.0, unit = "degC" }
$ fune set RW_X.spin_axis "0 1 0" --dry-run
…
+[attr]
+spin_axis = { x = 0.0, y = 1.0, z = 0.0 }
```

To check a unit before you use it, `fune units parse` shows the dimension and the SI value, and `fune units ls` lists every unit:

```console
$ fune units parse "18 mN*m*s"
 Input         Dimension         SI
────────────────────────────────────────────────
 18.0 mN·m·s   AngularMomentum   0.0180 m²·kg/s
```

**What is refused.** Nothing is written when the text is wrong, and the exit code is 1 or 2:

| You type | You get |
|---|---|
| A quantity with the wrong dimension: `fune set RW_X.mass "160 m"` | "Unit m is Length, but attribute mass is Mass" (`unit.dimension`) |
| A quantity with no unit: `fune set RW_X.mass 160` | "Unit  is Dimensionless, but attribute mass is Mass" |
| Text that is not a number: `fune set RW_X.mass abc` | "Cannot parse unit: abc (Cannot read value abc)" |
| A vector with two numbers: `fune set RW_X.spin_axis "0 1"` | "Cannot parse unit: 0 1 …" |
| A boolean that is not `true` or `false`: `fune set conditions.orbit.sun_synchronous maybe` | "Invalid input: Cannot read value maybe" |
| `--min` in another unit than the value: `--min "1 s"` | "The unit of --min / --max must be the same as the value: g" |
| An unknown source: `--source nope` | "Source nope is not in sources/" (`ref.source`) |
| An unknown attribute or part: `RW_X.nothing`, `NOPE.mass` | "attribute definition not found: nothing", "target not found: Part NOPE" |
| A word that is not in the enumeration: `analysis.aocs.law.NOMINAL.law foo` | "unknown variant foo, expected one of sun_pointing, nadir_pointing, inertial" |
| A tensor | Not possible as text. Pass it as JSON with `fune apply` |
| A limit that lives in a requirement: `fune setting set analysis.power.dod_max 0.3` | The message says that it is a verdict criterion held by a requirement, and which one to edit with `fune requirement set` |

Some mistakes are accepted and reported as a warning by `fune check`, because the file is valid: for example `--min "100 g" --max "90 g"` gives `value.uncertainty_order`, "The uncertainty does not satisfy min ≤ value ≤ max". Run `fune check` after an edit you are unsure of.

### Settings and records

`fune set` also writes settings. The path starts with `conditions.`, `analysis.` or `budget.`, for example `fune set conditions.orbit.altitude "550 km"` (the same as `fune setting set conditions.orbit.altitude "550 km"`). `fune setting ls` lists every setting with its value and file; `fune setting unset <PATH>` removes one.

Fields of an analysis case are written with `cases.<ID>.<field>`. The fields are `label`, `description`, `mode`, `phase`, `side` (`nominal`, `min` or `max`), `thermal_case`, `beta`, `orbits`, `sun_distance` and `epoch`. Because only one field changes, this is the way to edit a case ([Cases](#cases-phases-and-configurations)):

```console
$ fune set cases.hot_imaging.beta "45 deg" --dry-run
--- a/cases/hot_imaging.toml
+++ b/cases/hot_imaging.toml
@@ -2,5 +2,5 @@
 mode         = "MISSION"
 side         = "max"
 thermal_case = "hot"
-beta         = { value = 60.0, unit = "deg" }
+beta         = { value = 45.0, unit = "deg" }
 phase        = "NOMINAL_OPS"
```

`fune unset cases.<ID>.<field>` removes one field.

## Parts and catalog

| Command | What it does |
|---|---|
| `fune part add <ID> --catalog <ITEM> [--group <G>] [--count N] [--label L] [--mode-state S]` | Add a part from a catalog item. `--kind <KIND>` makes a blank part of a kind instead. A row is added to every mode, `off` by default |
| `fune part rm <ID>` | Remove a part and its rows in the modes. Refused, with the list of errors, while another file still refers to it |
| `fune part rename <OLD> <NEW>` | Rename a part. Every reference is rewritten |
| `fune part mv <ID> <TARGET>` | Move a part to another subsystem, group or assembly. `aocs/RW` moves it into the group `RW` under `aocs`, creating the group |
| `fune part rename-group` | Rename a folder |
| `fune part place <ID> --on <PART> --point <P> --position "x y z mm" --rotation "rx ry rz deg"` | Set the mount target and the relative position and attitude. `--body-pos` and `--body-rot` use vehicle coordinates. `--configuration <ID>` writes the pose of a configuration |
| `fune part point <PART> <POINT> --position "…"` | Add or remove (`--rm`) a mount point on a part |
| `fune catalog add <ID> --kind <KIND> [--label …] [--maturity …] [--from-part <PART>]` | Add a catalog item. The ID has an origin: `local/rw_30mNms` |
| `fune catalog set <ITEM> <PATH> <VALUE>` | Write an attribute or a field: `mass`, `power.on`, `label`, `manufacturer`, `maturity` |
| `fune catalog unset`, `rm`, `rename`, `mv` | Remove a value, remove an item (refused while a part uses it), rename it, or move it to another directory of the catalog |

An example. Add a part and see which files change:

```console
$ fune part add FSS_2 --catalog local/fss_60 --group aocs --dry-run
--- a/modes/DOWNLINK.toml
+++ b/modes/DOWNLINK.toml
@@ -5,6 +5,7 @@
 [states]
 CAM     = { state = "off" }
 EPT     = { state = "off" }
+FSS_2   = { state = "off" }
 GPS     = { state = "on" }
 …
--- a/parts/aocs/FSS_2.toml
+++ b/parts/aocs/FSS_2.toml
@@ -0,0 +1 @@
+catalog = "local/fss_60"
✔ dry-run: 5 files would change (nothing written)
```

The new part is a one-line file that points to the catalog item, and takes its mass, power and limits from it. Four mode files change too, because the part gets an `off` row in each mode. Rename and move keep every reference consistent. A dry run of a rename shows all the files it would touch:

```console
$ fune part rename GPS GPS_1 --dry-run
--- a/analysis/thermal.toml
+++ b/analysis/thermal.toml
@@ -32,7 +32,7 @@
 BAT      = ["BAT", "HTR_BAT"]
 BUS      = ["BUS_FRAME"]
 CAM      = ["CAM", "HTR_CAM"]
-ELEC     = ["OBC", "PCDU", "GPS", "UHF_TRX", "STX"]
+ELEC     = ["OBC", "PCDU", "GPS_1", "UHF_TRX", "STX"]
…
✔ dry-run: 6 files would change (nothing written)
```

Removing a part that others still refer to is refused. The refusal is the list of diagnostics that the removal would cause (for example "Mount target part `CAM` is not defined"), and nothing is written. Remove or re-point the referring files first, or use `fune part rename`.

## Modes and sequences

```console
$ fune mode table
 Part      NOMINAL           MISSION               DOWNLINK          SAFE
─────────────────────────────────────────────────────────────────────────────────────
 CAM       off               standby (on 50.0 %)   off               off
 EPT       off               off                   off               off
 GPS       on                on                    on                off
 HTR_BAT   off (on 20.0 %)   off (on 10.0 %)       off (on 10.0 %)   off (on 40.0 %)
 …
```

The text in brackets is a duty: the fraction of time in another state.

| Command | What it does |
|---|---|
| `fune mode add <ID> [--label …] [--from <MODE>]` | Add a mode. `--from` copies the table of another mode |
| `fune mode set <MODE> <PART> <STATE> [--duty state=fraction] [--active N] [--spare-state S]` | Set the state of a part in a mode. `--duty tx=0.1` means 10 % of the time in `tx` |
| `fune mode unset <MODE> <PART>` | Remove a part's row from a mode |
| `fune mode duration <MODE> --max-duration "90 min"` | Write a maximum duration (leave the flag out to remove it) |
| `fune mode timing <MODE> --when sunlit` | Write when the mode is used in the longest-duration evaluation. `--start eclipse_exit --offset "5 min"` is the other form |
| `fune mode move`, `rename`, `rm` | Reorder, rename, remove. Removing a mode that cases, sequences or phases still use is refused, and the message lists them |
| `fune mode view <SYSTEM>` | The per-mode values of `power`, `cdh`, `thermal` or `aocs`, sunlit and eclipse |

Example: put the camera in standby during the downlink (dry run):

```console
$ fune mode set DOWNLINK CAM standby --dry-run
--- a/modes/DOWNLINK.toml
+++ b/modes/DOWNLINK.toml
@@ -3,7 +3,7 @@
 max_duration = { value = 15.0, unit = "min" }

 [states]
-CAM     = { state = "off" }
+CAM     = { state = "standby" }
```

`fune mode view power` prints the power of each part in each mode, which is the table the Power page of the GUI shows:

```console
$ fune mode view power
Sunlit
 Part                            NOMINAL    MISSION    DOWNLINK   SAFE
───────────────────────────────────────────────────────────────────────────
 CAM                                  0 W     4.20 W        0 W        0 W
 EPT                                  0 W        0 W        0 W        0 W
 GPS                               900 mW     900 mW     900 mW        0 W
 …
```

**Sequences.** A named sequence is an ordered list of modes with durations. The power analysis simulates the one that is in use. `fune sequence ls` lists them, `fune sequence set` writes one in full, and `fune sequence rm` removes one:

```console
$ fune sequence ls
 Name       In use   Anchor                    Segments
───────────────────────────────────────────────────────────────────────────────────────────────────────────────
 default    yes                                NOMINAL 76.0 min, MISSION 5.00 min, NOMINAL 5.00 min, DOWNLINK 9.60 min
 downlink            eclipse_exit + 10.0 min   NOMINAL 60.0 min, DOWNLINK 15.0 min, NOMINAL 20.6 min
 imaging             eclipse_exit              NOMINAL 40.0 min, MISSION 10.0 min, NOMINAL 35.6 min, DOWNLINK 10.0 min
$ fune sequence set imaging_day --segments "NOMINAL=76 min,MISSION=5 min" --anchor eclipse_exit --dry-run
--- a/analysis/power.toml
+++ b/analysis/power.toml
@@ -57,3 +57,14 @@
…
+[sequences.imaging_day]
+anchor = { from = "eclipse_exit" }
…
```

To make a sequence the one in use: `fune setting set analysis.power.active_sequence imaging_day`.

## Cases, phases and configurations

Orbit and environment settings are written with `fune set conditions.<…>`, as in the [first-satellite tutorial](first-satellite.md#11-run-an-analysis-and-fix-the-missing-inputs).

**Cases.** `fune case set <ID>` creates a case, or *replaces* the whole record: fields you do not give are removed, and the command lists them. To change one field, use `fune set cases.<ID>.<field>`.

```console
$ fune case ls
 ID              Name                          Modes     Phase         File
────────────────────────────────────────────────────────────────────────────────────────
 cold_safe       Cold (safe mode)              SAFE      NOMINAL_OPS   cases/cold_safe.toml
 eclipse_worst   Worst-case eclipse (β = 0°)   NOMINAL   NOMINAL_OPS   cases/eclipse_worst.toml
 hot_imaging     Hot (imaging)                 MISSION   NOMINAL_OPS   cases/hot_imaging.toml
$ fune case set my_case --mode NOMINAL --label "My case" --dry-run
--- a/cases/my_case.toml
+++ b/cases/my_case.toml
@@ -0,0 +1,2 @@
+label = "My case"
+mode  = "NOMINAL"
✔ dry-run: 1 files would change (nothing written)
```

Fields of a case: `--label`, `--description`, `--mode`, `--phase`, `--side nominal|min|max`, `--thermal-case`, `--beta "60 deg"`, `--orbits`, `--sun-distance "0.7 AU"`, `--epoch <ISO 8601>`. Other case commands: `rename`, `rm`, `set-phase`, `unset-phase`, `set-epoch`, `unset-epoch`, and `from-eclipse`, `from-sun-distance`, `from-geometry`, which create cases from the worst days of an orbit.

Removing a case that is still referred to is refused, and the message says what refers to it:

```console
$ fune case rm hot_imaging
✖ Invalid input: Analysis case `hot_imaging` cannot be removed: referred to by `default_case` in analysis/common.toml; `quantify.cases` of requirement(s) R_STORAGE. Change those first
```

The same holds for phases, modes, configurations, sources, links, ground stations and catalog items. Each refusal names the files or the entities that refer to the thing, so you know what to change first.

**Phases.**

```console
$ fune phase ls
 #   ID            Name                          Duration   Modes                           Orbit      Law   Cases
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 1   LEOP          Launch and early operations     30.0 d   SAFE NOMINAL
 2   NOMINAL_OPS   Nominal operations               730 d   NOMINAL MISSION DOWNLINK SAFE                    cold_safe eclipse_worst hot_imaging
 3   DISPOSAL      Disposal                         180 d   SAFE NOMINAL                    override
```

`fune phase add|set|rm|rename` write the heading fields. The orbit and attitude-law overrides of a phase are written with `set-field`, `unset-field`, `copy-orbit`, `unset-orbit` and `unset-law`. `fune phase show <ID>` prints one phase. `fune phase rm NOMINAL_OPS` is refused while cases refer to the phase.

**Configurations.** A configuration is a named overlay of part poses, such as stowed and deployed. `fune configuration ls` lists the configurations and the parts each one moves. `add`, `set`, `rm`, `rename` and `unset-pose` edit them; the poses are written with `fune part place <PART> --configuration <ID>`.

```console
$ fune configuration ls
 ID       Name     Overridden parts
────────────────────────────────────
 STOWED   Stowed   SAP_PY
$ fune configuration show STOWED
Stowed (STOWED)
  description  Launch configuration: the deployable paddle SAP_PY is folded flat onto the +X face …
  overridden   SAP_PY
  file         configurations/STOWED.toml
```

## Requirements

`fune requirement add <ID>` needs a statement, a quantity (a dimension: `Mass`, `Power`, `Temperature`, …), a direction (`--kind at_most|at_least|equal|range`) and a bound. What verifies it is either `--budget mass|power|data`, or `--analysis <ID>` with `--check <key>`:

```console
$ fune requirement add R_TEST --statement "Test" --quantity Mass --kind at_most --value "7 kg" --budget mass --dry-run
--- a/requirements/R_TEST.toml
+++ b/requirements/R_TEST.toml
@@ -0,0 +1,9 @@
+statement = "Test"
+quantity  = "Mass"
+
+[constraint]
+kind  = "at_most"
+value = { value = 7.0, unit = "kg" }
+
+[verify]
+budget = "mass"
✔ dry-run: 1 files would change (nothing written)
requirement.source_missing

  ⚠ A requirement should name its source (a source ID in sources/)
```

The warning at the end asks for `--source <ID>`. Further options: `--statement-en`, `--label`, `--tolerance`, `--min`/`--max` (a range), `--all-cases true|false`, `--cases a,b`, `--input mission_duration`. `fune requirement set` replaces a whole requirement. `fune requirement rm` removes one.

**The compliance table** collects the verdicts. It exits with 1 on a violation:

```console
$ fune requirement compliance
 Verdict          ID               Requirement                                                   Constraint   Cases        Detail
 ✔                R_BAT_TEMP       The battery shall stay above 0 degC in every analysis case.      ≥ 0 °C   cold_safe, eclipse_worst, hot_imaging   cold_safe · BAT minimum temperature …
 ? not verified   R_DEPLOY         The antenna shall be deployed within 30 minutes after separation.   ≤ 30.0 min   default   no analysis check or budget linked
 …
 ✖                R_STORAGE        The data held on board shall not exceed the OBC storage of 8 GB.   ≤ 8.00 GB   eclipse_worst, hot_imaging   hot_imaging: Total 54.1 GB against limit 8.00 GB
16 requirements: 11 ok, 1 warning, 1 violation, 3 not verified
```

The columns are trimmed here, and the last line is the one to read.

## Links and ground stations

A **ground station** is a file in `conditions/ground/`. `fune ground add` writes its location (`--latitude`, `--longitude`, `--altitude`, `--min-elevation`); the performance of each radio band is written with `fune set conditions.ground.<ID>.link.<band>.<field>`. A latitude outside -90 to 90 deg or a minimum elevation outside 0 to 90 deg is refused:

```console
$ fune ground add TEST_GS --latitude "35.7 deg" --longitude "139.7 deg" --dry-run
--- a/conditions/ground/TEST_GS.toml
+++ b/conditions/ground/TEST_GS.toml
@@ -0,0 +1,2 @@
+latitude  = { value = 35.7, unit = "deg" }
+longitude = { value = 139.7, unit = "deg" }
$ fune ground add BAD --latitude "95 deg" --longitude "139.7 deg" --dry-run
✖ Nothing was written because of validation errors (1)
setting.range

  × Latitude must be from -90 to 90 deg
  help: parameter: conditions.ground.BAD.latitude  conditions/ground/BAD.toml:1
```

A **link** is a section of `analysis/comm.toml` that ties a transceiver part to a band of the stations. `fune link add <ID>` writes the fields that make it load, and the losses and the modulation are set with `fune set analysis.comm.link.<name>.<field>`:

```console
$ fune link add x_down --direction down --part STX --band s_band --dry-run
--- a/analysis/comm.toml
+++ b/analysis/comm.toml
@@ -35,3 +35,8 @@
…
+[link.x_down]
+direction = "down"
+part      = "STX"
+band      = "s_band"
$ fune set analysis.comm.link.s_down.pointing_loss "1.5 dB" --dry-run
…
-pointing_loss       = { value = 1.0, unit = "dB" }
+pointing_loss       = { value = 1.5, unit = "dB" }
```

`fune ground rename|rm` and `fune link rename|rm` keep the references (the list of stations the analysis uses, the downlink choice and the requirement checks) consistent. Removing a link or a station that a requirement check refers to is refused:

```console
$ fune link rm s_down
✖ Invalid input: Link `s_down` cannot be deleted: `verify.check` of requirement(s) R_LINK_S, R_PFD refers to it. Update or remove those requirements first
```

The stations an analysis uses are the list `analysis.comm.stations`.

## Maneuvers

Maneuvers are files in `maneuvers/`, in execution order. A maneuver needs a category (`insertion`, `station_keeping`, `unloading`, `deorbit`, `other`) and a method (`impulsive`, `finite_burn`, `low_thrust`, `propagated`):

```console
$ fune maneuver ls
 #   ID        Name                                Category    Method       Thruster   Target
───────────────────────────────────────────────────────────────────────────────────────────────────────
 1   RAISE     Orbit raise 550 → 600 km            Insertion   Low thrust   EPT        altitude 600 km
 2   DEORBIT   Deorbit (lower perigee to 250 km)   Deorbit     Impulsive    EPT        perigee_altitude 250 km
$ fune maneuver add TCM --category station_keeping --method impulsive --delta-v "15 m/s" --dry-run
--- a/maneuvers/TCM.toml
+++ b/maneuvers/TCM.toml
@@ -0,0 +1,3 @@
+category = "station_keeping"
+method   = "impulsive"
+delta_v  = { value = 15.0, unit = "m/s" }
```

`--thruster`, `--after`, `--altitude`, `--perigee-altitude`, `--apogee-altitude`, `--inclination` and `--body` describe the target. `fune maneuver set` replaces a maneuver, `rm` removes one (later maneuvers are re-linked to the one before it), `rename` renames and `set-window` writes the dates of a transfer from the date search.

## Nets, harnesses, definitions and diagram layout

These commands are used less often. Each edit takes `--dry-run`.

**Nets** are electrical connections. A net has a kind (`power`, `data` or `rf`) and optional fields such as the rail voltage:

```console
$ fune net add P28V --kind power --rail "28 V" --limit "2 A" --dry-run
--- a/nets/P28V.toml
+++ b/nets/P28V.toml
@@ -0,0 +1,3 @@
+kind  = "power"
+rail  = { value = 28.0, unit = "V" }
+limit = { value = 2.0, unit = "A" }
✔ dry-run: 1 files would change (nothing written)
```

`fune net connect <PART>.<PORT> <NET>` puts a port on a net, and `disconnect` takes it off. `fune net set` and `unset` change fields of a net, `replace` rewrites it, `rename` and `rm` keep references consistent.

**Harness segments** are physical cables between two ports. `fune harness set` writes a whole segment (give every field you want to keep):

```console
$ fune harness set H_TEST --from OBC.CAN1 --to GPS.DATA --nets CAN_BUS --length "0.2 m" --dry-run
--- a/harness/H_TEST.toml
+++ b/harness/H_TEST.toml
@@ -0,0 +1,4 @@
+from   = "OBC.CAN1"
+to     = "GPS.DATA"
+nets   = ["CAN_BUS"]
+length = { value = 0.2, unit = "m" }
✔ dry-run: 1 files would change (nothing written)
```

**Attribute and kind definitions.** `fune attr add` defines a new attribute (type, dimension, allowed units, and for enumerations the symbols). `fune kind add` defines a kind of part (states, required and optional attributes, port templates). Both also have `ls`, `replace`, `rename` and `rm`; the built-in ones cannot be replaced.

```console
$ fune attr add wheel_count --type scalar --dimension Dimensionless --label "Wheel count" --dry-run
--- a/defs/attribute/wheel_count.toml
+++ b/defs/attribute/wheel_count.toml
@@ -0,0 +1,3 @@
+label     = "Wheel count"
+type      = "scalar"
+dimension = "Dimensionless"
$ fune kind add cold_plate --extends component --states "off,on" --required "mass" --dry-run
--- a/defs/kind/cold_plate.toml
+++ b/defs/kind/cold_plate.toml
@@ -0,0 +1,4 @@
+label    = "cold_plate"
+extends  = "component"
+states   = ["off", "on"]
+required = ["mass"]
```

**Thermal tables.** `fune thermal tables` prints the thermal couplings between nodes with the value in use and where it comes from, the outer-surface coatings and the node temperatures, as the "Couplings and surfaces" tab of the GUI shows them. It only reads. Edit with `fune set analysis.thermal.conductive.<A>.<B>` (and `radiative`, `coating`, `surface`):

```console
$ fune thermal tables
Couplings
 Kind         Node A     Node B     Value        Source
─────────────────────────────────────────────────────────────
 conductive   BAT        BUS        0.0500 W/K   Written value
 conductive   BAT        PANEL_MZ   0.0300 W/K   Written value
 conductive   BUS        ELEC         2.50 W/K   Mount default
 …
```

**Acknowledged findings.** The geometry checks warn about overlaps between part boxes and about plumes or fields of view that touch a part. When a finding is a known artefact of the box model, acknowledge it with a reason. It then stays listed but no longer warns. `fune acknowledged rm` withdraws it:

```console
$ fune acknowledged add interference SAP_PY SP_PY --reason "Hinge edge of the box model" --dry-run
--- a/analysis/structure.toml
+++ b/analysis/structure.toml
@@ -22,4 +22,4 @@
 finding = "interference"
 first   = "SAP_PY"
 second  = "SP_PY"
-reason  = "Hinge edge: the paddle box and the solar panel box meet at the hinge line, where the real parts only touch"
+reason  = "Hinge edge of the box model"
```

(The demo already acknowledges this pair, so the dry run replaces the reason.)

**Diagram layout.** The system diagrams of the GUI place their nodes automatically. `fune view set <DIAGRAM> <NODE> --pos X,Y` fixes a node, `fune view edge-set` fixes the route of a line, `fune view ls <DIAGRAM>` lists what is fixed, and `fune view reset <DIAGRAM>` returns to automatic layout. The layout is stored in `view/`. Dragging a node in the GUI writes the same thing.

```console
$ fune view set electrical RW_X --pos 420,120 --dry-run
--- a/view/electrical.toml
+++ b/view/electrical.toml
@@ -0,0 +1,2 @@
+[nodes]
+RW_X = { x = 420, y = 120 }
✔ dry-run: 1 files would change (nothing written)
```

**Project settings.** `fune project set name <NAME>` and `fune project set label <TEXT>` write the `[project]` table of `fune.toml`; `fune project unset label` removes the label.

## Run analyses and budgets

`fune analyze ls` lists all analyses with their area, ID, name and kind. `fune analyze <ID>` runs one. The analyses are described in the [analysis pages](../analyses/index.md).

```console
$ fune analyze ls
 Area         ID                            Name                                           Kind          Input
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 aocs         aocs.rw_unloading             RW unloading frequency                         time series   built from design
 aocs         aocs.unload_time              Momentum unloading time                        steady        built from design
 …
$ fune analyze comm.link_budget
✔ comm.link_budget ran
     KPI                 Value
─────────────────────────────────
 ✔   Link margin (min)   3.05 dB
 Verdict   Target   Item                                 Detail
─────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok               s_down@TOKYO_GS link margin          Margin 5.95 dB (required at least 3.00 dB)
 …
Use --json for the full result
```

| Option | Meaning |
|---|---|
| `--case <ID>` | Run for an analysis case |
| `--configuration <ID>` | Build the input from a configuration (poses) |
| `--ops <FILE>` | Apply unsaved edits (a JSON file of Ops, or `-` for standard input) in memory, then analyse. Nothing is written |
| `--input <FILE>` | A JSON file that supplies the whole input, not built from the design |
| `--detail` | Also return the heavy optional output, in `--json` |
| `--json` | The full result: KPIs, checks, inputs, outputs, time series and missing inputs |

The exit code is 1 if an input is missing, the run fails or a check is a violation. When an input is missing, the command lists each one with the reason and the `fune set` command that fixes it ([tutorial, step 11](first-satellite.md#11-run-an-analysis-and-fix-the-missing-inputs)).

**Budgets.** `fune budget mass|power|data` prints a budget. `--case` counts power and data from the case's mode only. `--ops` applies unsaved edits:

```console
$ fune budget power
Power budget
power.mode_load  mode DOWNLINK
     Row             Current   With growth   Allocated   Margin
─────────────────────────────────────────────────────────────────
 ✔   AOCS             4.32 W        4.58 W      6.00 W    1.42 W
 …
 ✖   Communication    10.3 W        11.3 W      3.00 W   -8.27 W
 …
✔ Total 18.7 W  limit 24.0 W (system margin 15.0 %)  margin 1.68 W
```

**What-if without writing.** Describe the change as Ops in JSON, and look at the result:

```console
$ cat ops.json
[{"op":"set_value","part":"CAM","attr":"mass","value":{"type":"scalar","data":{"value":1.2,"unit":"kg"}}}]
$ fune budget mass --ops ops.json | tail -2
✔ Dry total 6.57 kg  limit 12.0 kg (system margin 10.0 %)  margin 4.23 kg
✔ Total with propellant 6.64 kg  margin 4.16 kg  (propellant from propulsion.propellant)
```

The files stay as they were. The GUI uses the same mechanism for its unsaved drafts.

## Apply Op JSON

`fune apply <FILE>` applies a file of Ops (use `-` for standard input). It is the same JSON that the GUI sends, and it suits scripts that make many edits at once:

```console
$ fune apply ops.json --dry-run
--- a/parts/payload/CAM.toml
+++ b/parts/payload/CAM.toml
@@ -8,6 +8,9 @@
 [pose]
 position = { x = 0.0, y = -60.0, z = -90.0, unit = "mm" }

+[attr]
+mass = { value = 1.2, unit = "kg" }
+
 [ports.PWR]
 net = "P5V"

✔ dry-run: 1 files would change (nothing written)
```

## Check, format, undo

```bash
fune check                   # exit 1 on an error
fune check --deny-warnings   # warnings fail too
fune fmt --check             # list files not in canonical form; exit 1 if any
fune fmt                     # rewrite them
```

`fune fmt` puts files in canonical form: a fixed key order, aligned `=`, TOML 1.0. Files that `fune` writes are already in canonical form, so `fmt --check` reports files only after you edited them by hand. `fune fmt` is not an Op, so it cannot be undone with `fune undo`.

### Undo and history

```console
$ fune history
   1          fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  1 file(s)
(keeps the last 100 edits; `fune undo` / `fune redo` move along this list)
$ fune undo
  modified parts/payload/CAM.toml
✔ Undid: fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  rev b3:5e54b14f210a
```

`fune history` lists the edits (oldest first). `fune undo` restores the files that the last edit changed. `fune redo` re-applies an undone edit, and a new edit clears the redo side. Undo is refused (exit code 3) if the files changed since, by an editor, git, `fune fmt` or a catalog fetch. The history is shared with the GUI, whose change-history popover has Undo and Redo buttons.

## Export the design document

`fune report export` writes the tables and figures of a design report for your own Typst document. It changes nothing in the design:

```console
$ fune report export --out exports/report
✔ Wrote 5 files to exports/report (revision b3:5e54b14f210a…)
```

It writes `data.json`, `iwafune.typ`, `example.typ` and, for the history items, a `figs/` directory of SVG figures. Options:

| Option | Meaning |
|---|---|
| `--out <DIR>` | Output directory. Default: `exports/report` under the project root |
| `--case <ID>` | An analysis case to export the case-dependent items for. Repeat it for several. Without it, the default case of the analyses is used once |
| `--item <NAME>` | An item to export: `mass`, `mode-power`, `link-budget`, `thermal-result`, `thermal-history`, `power-history`. Repeat it for several. Without it, all six |
| `--lang <ja\|en>` | Language of headings, captions and notes. Default: `en` |

```console
$ fune report export --out exports/mass --item mass --lang ja
✔ Wrote 3 files to exports/mass (revision b3:5e54b14f210a…)
```

A single table item writes three files and no figures. An unknown item is refused with the list of the valid ones. The details are in [Report export](../report.md).

## Data sets

```console
$ fune data status
 ID                 State               Size      Loaded   Location
─────────────────────────────────────────────────────────────────────────────────────────
 de440              ✔ SHA-256 matches   20.7 MB   yes      ~/.cache/iwafune/data/de440.bin
 …
 space_weather      not fetched               —   —        ~/.cache/iwafune/data/SW-All.csv
Fetch: fune data fetch space_weather
```

`fune data status` lists the optional data sets (ephemeris, space weather, radiation data) with their state, size and location. `fune data fetch <ID>` downloads one. `fune data space-weather <DATE>` looks up F10.7 and Ap for a date, and says whether the value comes from the downloaded table or from a preset. See [Install](install.md#optional-data-sets) for the table and the commands.

## Shared catalogs

A shared catalog is a git repository with a `catalog/` directory (and optionally `sources/`). To use one:

```console
$ fune catalog source add shared --git file:///srv/shared-catalog --rev main
  modified fune.toml
✔ Wrote 1 files  rev b3:d13744bad82d
catalog.not_locked

  × The commit of shared catalog `shared` is not in catalog.lock. Run `fune catalog fetch` to fetch it, then commit catalog.lock
…
Added shared catalog `shared` to fune.toml (not fetched yet). Run `fune catalog fetch`, then commit catalog.lock
$ fune catalog fetch
 Catalog   git                          rev    commit         Previous commit
───────────────────────────────────────────────────────────────────────────────
 shared    file:///srv/shared-catalog   main   668d80223e1a   —
+ shared/rw_10
✔ Rewrote catalog.lock (commit it)
$ fune catalog ls
 ID              Subsystem   Kind             Name                                          Model   Attributes
───────────────────────────────────────────────────────────────────────────────────────────────────────────────
 local/bat_2s    power       battery          Li-ion 2S pack                                                 3
 …
 shared/rw_10    aocs        reaction_wheel   Reaction wheel, 10 mN*m*s class (fictional)                    1
```

The repository path is shortened. Use an `https` or `ssh` URL for a real one. The URL can be anything that `git` can fetch. Authentication is whatever your git configuration provides.

The items are named `<name>/<ID>`, and read-only. Use them like any item: `fune part add RW_X --catalog shared/rw_10 --group aocs`. Commit `fune.toml` and `catalog.lock`. The commit is pinned in `catalog.lock`, and the fetched files are cached under `.fune/`.

To move to a newer commit of the repository, update the catalog. The command shows what changed:

```console
$ fune catalog update shared
 Catalog   git                          rev    commit         Previous commit
───────────────────────────────────────────────────────────────────────────────
 shared    file:///srv/shared-catalog   main   90cdb29c85f2   668d80223e1a
~ shared/rw_10
    attr.mass: 150 g → 160 g
✔ Rewrote catalog.lock (commit it)
```

`fune catalog source rm shared` removes a catalog, and is refused while a part uses one of its items:

```console
$ fune catalog source rm shared
✖ Invalid input: Items of catalog `shared` are used by part RW_T so it cannot be removed. Re-point or delete the users first
```

`fune catalog source add-local <NAME> --path <DIR>` adds another catalog that lives in this project, in a directory such as `vendor_parts` (not one the project layout uses, such as `parts`). Its items are named `<NAME>/<ID>`. The directory is created when you add the first item:

```console
$ fune catalog source add-local vendor --path vendor_parts --dry-run
--- a/fune.toml
+++ b/fune.toml
@@ -9,5 +9,9 @@
 name = "local"
 path = "catalog"

+[[catalog]]
+name = "vendor"
+path = "vendor_parts"
+
 [modes]
 order = ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]
✔ dry-run: 1 files would change (nothing written)
```

In CI, fetch the locked commit, without changing the lock, before you check:

```console
$ fune catalog fetch --locked
 Catalog   git                          rev    commit         Previous commit
───────────────────────────────────────────────────────────────────────────────
 shared    file:///srv/shared-catalog   main   668d80223e1a   (same)
✔ catalog.lock is unchanged
```

`--locked` fails if `catalog.lock` is missing or does not match `fune.toml`. For a private repository, let git authenticate with a token before `fune catalog fetch --locked`:

```yaml
      - uses: actions/checkout@v4
      - run: git config --global url."https://x-access-token:${{ secrets.CATALOG_TOKEN }}@github.com/".insteadOf "https://github.com/"
      - run: fune catalog fetch --locked
      - run: fune check
```

## Agent skill, updates and the GUI

**Agent skill.** `fune skill` teaches an AI coding agent how to work on a design. `fune skill show` prints the instruction text (`SKILL.md`) that ships inside the binary. `fune skill install` writes it into the project:

```console
$ fune skill show | head -4
---
name: fune
description: Edit and analyze satellite conceptual-design data with the `fune` CLI (a git-managed directory of TOML with a fune.toml at its root). Use it whenever you add or change parts, modes, ...
metadata:
$ fune skill install
✔ Installed the fune skill …: my-demo/.claude/skills/fune/SKILL.md
$ fune skill install
✔ The fune skill is up to date (…): my-demo/.claude/skills/fune/SKILL.md
```

Installing again does nothing if the file is current, and replaces it if another version wrote it. It refuses (exit code 3) if you edited it, unless you add `--force`. `--skills-dir <PATH>` installs into another skills directory. The text says to edit only through the CLI, to run `fune check` and `fune fmt --check`, and to write units.

**Update the program.** `fune self-update --check` reports the current and the latest version and changes nothing. `fune self-update` installs the latest release, and `fune self-update --version <MAJOR.MINOR.PATCH>` installs exactly that version. It needs `curl` and `tar` and works outside a project. See [Install](install.md#update).

```console
$ fune self-update --check
Checking the latest release...
✔ fune … is already the latest version
```

**Start the GUI.** `fune gui` starts a local web server on the project and opens the browser. See the [GUI guide](gui.md).

| Option | Meaning |
|---|---|
| `--port <PORT>` | Port (default 3000; if busy and not given, the next free port) |
| `--host <HOST>` | Address to listen on (default `127.0.0.1`; `0.0.0.0` for all interfaces) |
| `--allow-host <NAME>` | A name or IP that the server accepts in the `Host` header (repeatable). Needed when you reach the server by another name. Requests with another `Host` get a 403 |
| `--no-open` | Do not open the browser |

## Command reference

One line per command, from `fune --help` and the help of each subcommand. Every command also takes the [global options](#global-options). Edits take `--dry-run` and `--base-revision`.

| Command | Purpose |
|---|---|
| `fune init [DIR]` | Create a new project (`--demo`: the bundled demo satellite) |
| `fune check` | Check the design data (exit code 1 on errors); `--deny-warnings` |
| `fune gui` | Start the GUI (a local HTTP server) and open it in the browser |
| `fune status` | Project overview |
| `fune part ls\|show\|add\|rm\|mv\|rename\|rename-group\|place\|point` | Parts |
| `fune set <PATH> <VALUE>` | Set a value (`--source`, `--at`, `--note`, `--min`, `--max`) |
| `fune get <PATH>` | Show a value (`--state`) |
| `fune unset <PATH>` | Remove an overridden value and revert to the catalog value; also removes one field of an analysis case |
| `fune attr ls\|add\|replace\|rename\|rm` | Attribute definitions |
| `fune kind ls\|add\|replace\|rename\|rm` | Kind definitions |
| `fune mode ls\|table\|view\|add\|rm\|rename\|move\|duration\|timing\|set\|unset` | Operating modes |
| `fune thermal tables` | Thermal couplings, outer-surface coatings and coatings (read; edit with `fune set\|unset analysis.thermal.…`) |
| `fune sequence ls\|set\|rm` | Named mode sequences of the power analysis |
| `fune acknowledged add\|rm` | Acknowledged geometry findings: overlaps and plume or field-of-view hits that are known box artefacts |
| `fune net ls\|add\|set\|unset\|replace\|rm\|rename\|connect\|disconnect` | Nets (electrical connections) |
| `fune setting ls\|set\|unset` | Settings of `conditions/`, `analysis/` and `budget/` |
| `fune catalog ls\|show\|fetch\|update\|add\|set\|unset\|rm\|rename\|mv\|source` | Catalog items and the catalog origins (`source add\|add-local\|rm`) |
| `fune case ls\|set\|from-eclipse\|from-sun-distance\|from-geometry\|set-phase\|unset-phase\|set-epoch\|unset-epoch\|rm\|rename` | Analysis cases |
| `fune source ls\|set\|rm\|rename\|mv` | Source register |
| `fune harness ls\|set\|rm\|rename` | Harness segments |
| `fune maneuver ls\|add\|set\|rm\|rename\|set-window` | Maneuvers |
| `fune phase ls\|show\|add\|set\|rm\|rename\|set-field\|unset-field\|copy-orbit\|unset-orbit\|unset-law` | Mission phases |
| `fune configuration ls\|show\|add\|set\|rm\|rename\|unset-pose` | Configurations: named overlays of part poses, e.g. stowed and deployed |
| `fune ground add\|rm\|rename` | Ground stations |
| `fune link add\|rm\|rename` | Communication links |
| `fune project set\|unset` | Project settings (`name`, `label` in `fune.toml`) |
| `fune requirement ls\|compliance\|add\|set\|rm` | Requirements and the compliance table |
| `fune view ls\|set\|rm\|reset\|edge-set\|edge-reset` | Diagram layout (node positions of the system diagrams) |
| `fune units ls\|parse\|format` | Units |
| `fune fmt` | Format the design files; `--check` only reports |
| `fune history` | List the edit history that `fune undo` and `fune redo` use (oldest first) |
| `fune undo` | Undo the last edit |
| `fune redo` | Redo the edit that was undone |
| `fune analyze ls\|<ID>` | Run an analysis (`--case`, `--configuration`, `--input`, `--ops`, `--detail`) |
| `fune budget mass\|power\|data` | Budgets (`--case`, `--ops`) |
| `fune report export` | Report tables and figures for a Typst design document (`--out`, `--case`, `--item`, `--lang`) |
| `fune data status\|fetch\|space-weather` | External data (ephemeris, space weather, radiation data) |
| `fune self-update` | Replace this binary with the latest release (`--check`, `--version`) |
| `fune skill show\|install` | Show or install the instructions that teach an AI agent to use `fune` |
| `fune apply <FILE>` | Apply Op JSON (the same form the GUI uses) |
