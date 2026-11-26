# Build your first satellite

This tutorial takes you from an empty directory to a mass budget, a power budget and one analysis verdict. You create a project and build a minimal satellite: an on-board computer, a battery and a solar panel on a bus structure. On the way you see how to fix "missing inputs" one at a time.

You do everything on the command line, because the commands show exactly what changes. Each command has an equivalent in the GUI; the [GUI guide](gui.md) says where. You can run `fune gui` at any step to look at the result. The tutorial takes about 30 minutes.

You need the `fune` command and `git` ([install guide](install.md)). The values are made up. Replace them with your own. Output is shortened with `…` where it is long, and your revision hashes (`rev b3:…`) will differ in some steps.

## 1. Create the project

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
$ cd my-sat
$ git init
```

`fune init` does not run `git init`, so do it yourself. A new project has five files:

- `fune.toml` holds the project name and the list of catalogs. A project starts with one catalog of its own, `local`, in the `catalog/` directory.
- `budget/allocation.toml` and `budget/policy.toml` hold the allocations and the margin policy.
- `modes/SAFE.toml` is a first operating mode.
- `.gitignore` keeps the cache directory `.fune/` out of git.

```console
$ fune status
my-sat
my-sat/.  branch main  rev b3:2ef6b7d3b
parts 0  groups 0  modes 1  nets 0  harnesses 0  cases 0
⚠ Check: 1 warnings
$ fune check
budget.limit_unset

  ⚠ No requirement sets the mass limit (add an at_most requirement with verify.budget = "mass")
   ╭─[budget/allocation.toml:1:1]
 1 │ [mass]
   · ───┬──
   ·    ╰── requirement[budget/mass]
 2 │ system_margin = 0.1
   ╰────
  help: parameter: requirement[budget/mass]  budget/allocation.toml:1
⚠ Check: 1 warnings
```

What you should see: `fune check` already has one warning. It says that nothing sets the mass limit. In iwafune, a limit is a requirement, so you add one in step 3. First you need a source to cite.

## 2. Register sources

Every value should say where it came from. A source is a small file in `sources/`. There are five kinds: `placeholder`, `judgement`, `analysis`, `datasheet` and `measured`. Register two:

```console
$ fune source set sys_judgement --kind judgement --label "Team estimate" --date 2026-11-01
  created sources/sys_judgement.toml
✔ Wrote 1 files  rev b3:50d890c01f76
$ fune source set ds_vendor --kind datasheet --label "Vendor datasheets (fictional)" --ref "docs/datasheets/"
  created sources/ds_vendor.toml
✔ Wrote 1 files  rev b3:a9b9490b4858
```

What you should see: each command prints `created` and the file name, and `✔ Wrote 1 files` with a revision. After an edit that leaves a warning, `fune` prints the warning again. The repeats are trimmed here. You can check the result of any step with `fune fmt --check` and `fune check`: after every command in this tutorial the first prints nothing, and the second reports no problems or only the warning that the next step fixes.

## 3. Add a mass requirement

```console
$ fune requirement add R_MASS --statement "The satellite total mass shall not exceed 3 kg." \
    --quantity Mass --kind at_most --value "3 kg" --budget mass --source sys_judgement
  created requirements/R_MASS.toml
✔ Wrote 1 files  rev b3:3e6153e4dbef
$ fune check
✔ Check: no problems
```

What you should see: `fune check` now reports no problems. The requirement says what it constrains (`--quantity Mass`), the direction (`at_most`), the value, and what verifies it (`--budget mass`: the mass budget). The warning is gone. A requirement of this form is the limit of the mass budget.

## 4. Fill the catalog

A catalog item describes a kind of component, such as a battery model, once. Parts then point to it. Create three items. The `local/` prefix is the catalog; the rest is the item name.

```console
$ fune catalog add local/bat_2s --kind battery --label "Li-ion 2S pack" --maturity off_the_shelf
$ fune catalog set local/bat_2s mass "250 g" --source ds_vendor --at "p.3 table 1"
$ fune catalog set local/bat_2s capacity "40 Wh" --source ds_vendor
$ fune catalog set local/bat_2s voltage "7.4 V" --source ds_vendor
```

Each command prints `created` or `modified` with the file name and `✔ Wrote 1 files`; we leave that out below. Note the parts of a value: a number, a unit (`g`, `Wh`, `V`), a source (`--source`), and optionally a place in the document (`--at`). Units are checked: `fune` rejects a unit with the wrong dimension. `fune catalog set local/bat_2s capacity "40 kg"` stops with the error "Unit kg is Mass, but attribute capacity is Energy" and writes nothing.

How do you know which attributes a kind needs? Ask for the kinds:

```console
$ fune kind ls
 ID                    role                  States                   Required attributes
─────────────────────────────────────────────────────────────────────────────────────────────────────
 battery               battery                                        capacity, voltage
 obc                   obc                   off / on                 power
 solar_panel           solar_panel                                    area, cell_efficiency, normal
 structure             structure
 ...
```

`fune check` also warns about each required attribute that is missing. (The table has more rows and one more column, `Definition`, than shown.) Now the solar panel and the computer:

```console
$ fune catalog add local/sp_body --kind solar_panel --label "Body-mounted solar panel" --maturity modified
$ fune catalog set local/sp_body mass "180 g" --source ds_vendor
$ fune catalog set local/sp_body area "0.06 m^2" --source ds_vendor
$ fune catalog set local/sp_body cell_efficiency 0.28 --source ds_vendor
$ fune catalog set local/sp_body normal "0 0 1" --source ds_vendor
$ fune catalog add local/obc_a --kind obc --label "On-board computer" --maturity off_the_shelf
$ fune catalog set local/obc_a mass "90 g" --source ds_vendor
$ fune catalog set local/obc_a power.off "0 W" --source ds_vendor
$ fune catalog set local/obc_a power.on "0.65 W" --source ds_vendor
$ fune catalog ls
 ID              Subsystem   Kind          Name                       Model   Attributes
─────────────────────────────────────────────────────────────────────────────────────────
 local/bat_2s    power       battery       Li-ion 2S pack                              3
 local/obc_a     cdh         obc           On-board computer                           2
 local/sp_body   power       solar_panel   Body-mounted solar panel                    4
```

What you should see: three items in `fune catalog ls`, with 3, 2 and 4 attributes. Three things to notice:

- A quantity with no unit, like `cell_efficiency`, is written as a plain number. A vector, like `normal`, is three numbers: `"0 0 1"`.
- `power.on` is a value *by state*. The power of a computer depends on whether it is off or on. The path is `<attribute>.<state>`.
- `--maturity` says how new the design is. The budget adds a growth margin by maturity: 5 % for `off_the_shelf`, 10 % for `modified`, 20 % for `new` (in `budget/policy.toml`).

## 5. Define a mode

A mode is how the satellite operates, for example nominal or safe. Add a nominal mode before you add parts, because every new part gets a row in every existing mode. You should see `created modes/NOMINAL.toml`.

```console
$ fune mode add NOMINAL --label "Nominal"
  created modes/NOMINAL.toml
✔ Wrote 1 files  rev b3:4c6979c3a743
```

## 6. Place the parts

A part is one use of a catalog item (or of a bare kind) in your satellite. Start with the structure. It has no catalog item, so use `--kind`:

```console
$ fune part add BUS --kind structure --group structure --label "Bus structure"
  created parts/structure/BUS.toml
✔ Wrote 1 files  rev b3:fb7bb40a8ec3
attr.missing

  ⚠ Required attribute `mass` is not set (kind `component`)
  help: parameter: BUS.mass  parts/structure/BUS.toml
$ fune set BUS.mass "600 g" --source sys_judgement --note "Placeholder until CAD"
  modified parts/structure/BUS.toml
✔ Wrote 1 files  rev b3:b29db88132ab
$ fune part point BUS FACE_PZ --position "0 0 50 mm"
  modified parts/structure/BUS.toml
✔ Wrote 1 files  rev b3:6d08861fb0cc
```

`--group` puts the part in a directory under `parts/`. The top-level directory is the subsystem (`power`, `cdh`, `structure`). `fune part point` adds a *mount point* to the bus: a named place where other parts attach. Now the three catalog parts, and where they sit:

```console
$ fune part add BAT --catalog local/bat_2s --group power
$ fune part add SP_PZ --catalog local/sp_body --group power --label "Solar panel +Z"
$ fune part add OBC --catalog local/obc_a --group cdh
  modified modes/NOMINAL.toml
  modified modes/SAFE.toml
  created parts/cdh/OBC.toml
✔ Wrote 3 files  rev b3:c310d1dba050
$ fune part place SP_PZ --on BUS --point FACE_PZ
$ fune part place BAT --on BUS --position "0 0 -20 mm"
$ fune part place OBC --on BUS --position "0 0 10 mm"
$ fune part ls
 ID      Subsystem   Kind          Definition      Count   Mass     Power         Operating temp
─────────────────────────────────────────────────────────────────────────────────────────────────
 BAT     power       battery       local/bat_2s        1    250 g             —   not set
 BUS     structure   structure     —                   1    600 g             —   not set
 OBC     cdh         obc           local/obc_a         1   90.0 g   650 mW (on)   not set
 SP_PZ   power       solar_panel   local/sp_body       1    180 g             —   not set
4 parts
```

What you should see: four parts in `fune part ls`, and no warnings from `fune check`. Adding `OBC` modified two mode files: a part gets an `off` row in every mode (you can pick another state with `--mode-state`). The part's mass comes from its catalog item. Positions are relative to the mount, in the part's own frame; open the Layout page of the GUI to see the boxes.

<!-- screenshot: /design/layout — the 3D view of the bus with the three parts placed and the Parts tree open -->
*The Layout page with the bus and the three parts.*

## 7. Override a value on one part

A catalog value is the default. A part can override it, and the override can carry an uncertainty. Say the computer's mass is 95 g, between 90 g and 110 g:

```console
$ fune set OBC.mass "95 g" --source sys_judgement --note "Margin for the connector" --min "90 g" --max "110 g"
  modified parts/cdh/OBC.toml
✔ Wrote 1 files  rev b3:b1f70c6a7c72
$ fune get OBC.mass
95.0 g (90.0 g … 110 g)  Override, sys_judgement, Judgement
```

What you should see: `fune get` shows the new value with its range. It also shows where the value comes from: here an override on the part, with its source. `fune unset OBC.mass` would remove the override and bring back the catalog value of 90 g.

## 8. Set the part states in each mode

```console
$ fune mode set NOMINAL OBC on
$ fune mode set SAFE OBC on
$ fune mode table
 Part   NOMINAL   SAFE
───────────────────────
 OBC    on        on
```

What you should see: a table with one row, `OBC`, and `on` in both modes. The mode table says which part is in which state in which mode. The battery and the panel have no states in this model, so they have no rows.

## 9. Read the mass budget

```console
$ fune budget mass
Mass budget
budget
     Row         Current   With growth   Allocated   Margin
────────────────────────────────────────────────────────────
 ✔   Power         430 g         461 g           —        —
 ✔   Structure     600 g         600 g           —        —
 ✔   C&DH         95.0 g        99.8 g           —        —
✔ Dry total 1.16 kg  limit 3.00 kg (system margin 10.0 %)  margin 1.54 kg
Propellant not counted: no propulsion (maneuvers or propulsion parts)
```

What you should see: a dry total of 1.16 kg under the 3.00 kg limit. The columns are: the current sum of the nominal values, the sum with growth margin by maturity, the allocation of the subsystem (none yet), and the margin. The footer compares the total with the limit from `R_MASS` and the system margin of 10 %. You have your first mass budget.

## 10. Add a power requirement and read the power budget

```console
$ fune requirement add R_POWER --statement "The orbit-average power consumption shall not exceed 5 W." \
    --quantity Power --kind at_most --value "5 W" --budget power --source sys_judgement
```

The power budget counts one mode at a time. Without a case, `fune budget power` picks a mode itself (in this project it picked `SAFE`). To say which mode you mean, add an analysis case that names it, and make that case the default:

```console
$ fune case set nominal_day --mode NOMINAL --label "Nominal day"
  created cases/nominal_day.toml
✔ Wrote 1 files  rev b3:2ede95432446
$ fune setting set analysis.common.default_case nominal_day
  created analysis/common.toml
✔ Wrote 1 files  rev b3:c493ce2fbd46
$ fune budget power --case nominal_day
Power budget
power.mode_load  case nominal_day  mode NOMINAL
     Row    Current   With growth   Allocated   Margin
───────────────────────────────────────────────────────
 ✔   C&DH    650 mW        683 mW           —        —
✔ Total 683 mW  limit 5.00 W (system margin 0 %)  margin 4.32 W
⚠ budget/allocation.toml has no [power]
```

A case is a named set of conditions for analyses: here only the mode. The last line is a hint that you have not allocated power to subsystems.

## 11. Run an analysis and fix the missing inputs

Run the energy balance of the power system:

```console
$ fune analyze power.energy_balance
✖ power.energy_balance: missing inputs (3)
 Missing input                           Reason
────────────────────────────────────────────────────────────────────────────────────────────────────────
 analysis.aocs.law.NOMINAL               Cannot pick the attitude law of the mode uniquely. Specify the attitude law of the target mode
 conditions.orbit                        Orbit settings (conditions/orbit.toml)
 requirement[power.energy_balance/dod]   Allowed DoD (an at_most requirement verified by power.energy_balance / dod)
To fix (replace the <…> parts):
  analysis.aocs.law.NOMINAL    fune set analysis.aocs.law.NOMINAL "<value>"
  conditions.orbit             fune set conditions.orbit "<value>"
```

The analysis did not run, and it did not guess. It tells you what it needs: here the orbit, the attitude law of the `NOMINAL` mode and a limit. The exit code is 1, so a script notices. Fix the items one by one.

**The orbit.** The path `conditions.orbit.<key>` writes one key of `conditions/orbit.toml`. A circular sun-synchronous orbit at 550 km needs these:

```console
$ fune set conditions.orbit.altitude "550 km"
$ fune set conditions.orbit.epoch "2027-04-01T00:00:00Z"
$ fune set conditions.orbit.eccentricity 0
$ fune set conditions.orbit.sun_synchronous true
$ fune set conditions.orbit.ltdn "10:30"
```

You can type `0` for the eccentricity: `fune` knows that the setting is a real number and writes `eccentricity = 0.0` to `conditions/orbit.toml`.

**The attitude law.** The power analysis needs to know where the panel points. Say the nominal mode points a body axis at the Sun. The key names are the same as the ones in the demo's `analysis/aocs.toml`:

```console
$ fune set analysis.aocs.law.NOMINAL.law sun_pointing
$ fune set analysis.aocs.law.NOMINAL.axis "1 0 0"
$ fune set analysis.aocs.law.NOMINAL.secondary "0 0 1"
```

**The depth-of-discharge limit.** A limit is a requirement. This one is verified by the `dod` check of `power.energy_balance`, so the analysis reads its limit from it:

```console
$ fune requirement add R_DOD --statement "The battery depth of discharge shall not exceed 30 % in the nominal mode." \
    --quantity Fraction --kind at_most --value "30 %" --analysis power.energy_balance --check dod \
    --source sys_judgement --all-cases false
$ fune analyze power.energy_balance
✔ power.energy_balance ran
     KPI                                    Value
────────────────────────────────────────────────────
 ✖   Power balance (orbit average, worst)   -682 mW
 ✔   Max DoD                                 8.16 %
 Verdict       Target    Item                     Detail
───────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok          Nominal   DoD limit                Max DoD 8.16 % (limit 30.0 %, when sustained over the whole analysis span)
 ✖ violation   Nominal   Orbital energy balance   Energy margin per orbit -1.09 Wh (including battery efficiency, ...)
 ✔ ok          SAFE      DoD limit                Max DoD 8.16 % (limit 30.0 %, when sustained over the whole analysis span)
 ✖ violation   SAFE      Orbital energy balance   Energy margin per orbit -1.09 Wh (including battery efficiency, ...)
Use --json for the full result
```

The analysis runs now, and it gives a real verdict: a **violation**. The satellite loses energy every orbit. Why? The solar panel's normal is `0 0 1` (body +Z), but the attitude law points body +X at the Sun, so the panel is edge-on. Fix the design, not the analysis: point +Z at the Sun.

```console
$ fune set analysis.aocs.law.NOMINAL.axis "0 0 1"
$ fune set analysis.aocs.law.NOMINAL.secondary "1 0 0"
$ fune analyze power.energy_balance --case nominal_day
✔ power.energy_balance ran
     KPI                                    Value
────────────────────────────────────────────────────
 ✔   Power balance (orbit average, worst)    13.9 W
 ✔   Max DoD                                0.979 %
 Verdict   Target    Item                     Detail
───────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok      Nominal   DoD limit                Max DoD 0.979 % (limit 30.0 %, when sustained over the whole analysis span)
 ✔ ok      Nominal   Orbital energy balance   Energy margin per orbit 22.1 Wh (including battery efficiency, ...)
Use --json for the full result
```

Both verdicts are `ok`. With `--case nominal_day` the analysis looks at that case's mode only. You have a first analysis verdict.

In the GUI, the same missing inputs appear as rows on the Power page, each with a button such as "Enter value" or "Add a part" that opens a small dialog. See [Missing inputs](gui.md#missing-input-rows-and-their-fix-actions).

## 12. Check the requirements

```console
$ fune requirement compliance
 Verdict   ID        Requirement                                                                 Constraint   Cases     Detail
──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔         R_DOD     The battery depth of discharge shall not exceed 30 % in the nominal mode.     ≤ 30.0 %   default   DoD limit (Nominal): Max DoD 0.979 % (limit 30.0 %, ...)
 ✔         R_MASS    The satellite total mass shall not exceed 3 kg.                              ≤ 3.00 kg   default   Total 1.16 kg against limit 3.00 kg
 ✔         R_POWER   The orbit-average power consumption shall not exceed 5 W.                     ≤ 5.00 W   default   Total 683 mW against limit 5.00 W
3 requirements: 3 ok, 0 warning, 0 violation, 0 not verified
```

Each requirement takes its verdict from the budget or the analysis check that verifies it.

## 13. Format, check and commit

```console
$ fune check
✔ Check: no problems
$ fune fmt --check
$ git add -A
$ git commit -m "First satellite: bus, battery, solar panel, computer"
```

`fune fmt --check` prints nothing and exits with 0: every file is in canonical form, because `fune` writes files that way. If you edit a file by hand, the command lists the files that differ and exits with 1; `fune fmt` rewrites them. Run `fmt --check` and `check` before every commit, and in CI.

If a command did something you did not want, `fune undo` reverses the last edit and `fune redo` brings it back. `fune history` lists the edits ([CLI guide](cli.md#undo-and-history)).

## What you built

```text
my-sat/
├── fune.toml
├── analysis/
│   ├── aocs.toml            attitude law of the nominal mode
│   └── common.toml          the default case
├── budget/
│   ├── allocation.toml
│   └── policy.toml
├── cases/
│   └── nominal_day.toml
├── catalog/
│   ├── bat_2s.toml
│   ├── obc_a.toml
│   └── sp_body.toml
├── conditions/
│   └── orbit.toml
├── modes/
│   ├── NOMINAL.toml
│   └── SAFE.toml
├── parts/
│   ├── cdh/OBC.toml
│   ├── power/BAT.toml
│   ├── power/SP_PZ.toml
│   └── structure/BUS.toml
├── requirements/
│   ├── R_DOD.toml
│   ├── R_MASS.toml
│   └── R_POWER.toml
└── sources/
    ├── ds_vendor.toml
    └── sys_judgement.toml
```

(`.gitignore` and the ignored cache `.fune/` are not shown.) Two files, as `fune` wrote them. First, the computer: a part that points to a catalog item and overrides one value.

```toml
# parts/cdh/OBC.toml
catalog = "local/obc_a"

[mount]
on = "BUS"

[pose]
position = { x = 0.0, y = 0.0, z = 10.0, unit = "mm" }

[attr.mass]
value       = 95.0
min         = 90.0
max         = 110.0
unit        = "g"
from.source = "sys_judgement"
from.note   = "Margin for the connector"
```

Second, the catalog item that it points to:

```toml
# catalog/obc_a.toml
kind     = "obc"
label    = "On-board computer"
maturity = "off_the_shelf"

[attr.mass]
value       = 90.0
unit        = "g"
from.source = "ds_vendor"

[attr.power]
off.value       = 0.0
off.unit        = "W"
off.from.source = "ds_vendor"
on.value        = 0.65
on.unit         = "W"
on.from.source  = "ds_vendor"
```

And one requirement, which carries the criterion of an analysis:

```toml
# requirements/R_DOD.toml
statement = "The battery depth of discharge shall not exceed 30 % in the nominal mode."
quantity  = "Fraction"
source    = "sys_judgement"

[constraint]
kind  = "at_most"
value = { value = 30.0, unit = "%" }

[verify]
analysis = "power.energy_balance"
check    = "dod"
```

Every file is a few lines. Compare these with the commands you typed. The format of every file is in the [data model](../model.md).

## Next steps

- Add the other subsystems. Each [analysis page](../analyses/index.md) says which parts, settings and requirements that analysis needs. The `fune analyze` messages and `fune check` warnings point you to the next missing piece.
- Learn the vocabulary: [Concepts](concepts.md).
- Look at a full project: the demo in `fixtures/demo-6u`, as in the [quickstart](quickstart.md).
