# Quickstart: a 10-minute tour

This tour takes about ten minutes. You copy the demo satellite, check it, look at it in the GUI, change one value, watch a budget turn red, run an analysis, read the change in git, and undo it. You need `fune` installed ([Install](install.md)) and git.

Each step says what to do and what you should see. A line with only `…` means that output was trimmed. Your numbers will match, because the demo is fixed data.

## 1. Get the demo

The demo is a fictional 6U satellite that lives in the repository, in `fixtures/demo-6u`. Copy it, so you can edit it freely, and put the copy under git so you can see your changes later:

```bash
git clone https://github.com/akira-nkjm/iwafune
cp -r iwafune/fixtures/demo-6u my-demo
cd my-demo
git init -b main
git add -A
git commit -m "Baseline demo"
```

You should now have a directory `my-demo` with one commit. It is a project: a directory with a `fune.toml` file. `fune` finds the project by looking for that file in the current directory and in the directories above it.

## 2. Check it

```console
$ fune status
iwafune-6u-demo
/path/to/my-demo/.  branch main  rev b3:5e54b14f2
parts 38  groups 11  modes 4  nets 8  harnesses 2  cases 3
✔ Check: no problems
$ fune check
✔ Check: no problems
```

Both commands should end with `✔ Check: no problems`.

`fune status` counts what the project holds. `fune check` validates it: units and dimensions, references to IDs that do not exist, missing required values. It exits with code 1 if it finds an error, so you can use it in scripts and CI. The `rev` is a hash of the design files. It changes when any file changes.

## 3. Open the GUI

```console
$ fune gui
✔ Started the GUI: http://localhost:3000/
Project: /path/to/my-demo  (Ctrl-C to quit)
```

The browser opens on the Overview page. If port 3000 is busy, `fune gui` uses the next free port and prints it. Use `--no-open` if you do not want the browser to open, and `--port` to choose a port. Leave the command running, and use a second terminal for the commands below. Press Ctrl-C in the first terminal to stop it when you are done.

<!-- screenshot: / as budgets — the Overview page of the demo: KPI cards, the Mass budget and Power budget panels, and the Diagnostics panel -->
*The Overview page of the demo project.*

## 4. Read the Overview page

The Overview page is the dashboard. From the top:

- **KPI cards** such as "Mass margin". Each card shows a value, a bar and a verdict colour. Click a card to open the subsystem page behind it.
- **Budget panels**, for example "Mass budget". Each row is a subsystem with its current value, the value with growth margin, the allocation and the margin. A red row is over its allocation.
- **Diagnostics**, the same counts as `fune check`: errors, warnings and info.

Find the Payload row in the mass budget: 920 g current, 1.10 kg with growth, 1.20 kg allocated, 96.0 g margin. You will change it next.

The sidebar on the left lists every page. The [GUI guide](gui.md) goes through them one by one.

## 5. Change one value

The camera `CAM` weighs 920 g. Suppose the lens barrel turns out heavier, 1.2 kg.

**In the GUI.** Open Components in the sidebar, then choose `CAM` (click it in the tree or the table, or press `Ctrl+K` and search for it). In the detail panel, find the Mass row and click its value. Type `1.2 kg` and press Enter. The value is saved at once. The change appears in "Change history" at the bottom of the sidebar, together with the command that makes the same edit:

```text
fune set CAM.mass "1.2 kg"
```

**On the command line.** This is the same edit, with a source and a note. Add `--dry-run` first to see the file diff without writing anything:

```console
$ fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel" --dry-run
--- a/parts/payload/CAM.toml
+++ b/parts/payload/CAM.toml
@@ -8,6 +8,12 @@
 [pose]
 position = { x = 0.0, y = -60.0, z = -90.0, unit = "mm" }

+[attr.mass]
+value       = 1.2
+unit        = "kg"
+from.source = "em_test_2026_10"
+from.note   = "Heavier lens barrel"
+
 [ports.PWR]
 net = "P5V"

✔ dry-run: 1 files would change (nothing written)
$ fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"
  modified parts/payload/CAM.toml
✔ Wrote 1 files  rev b3:3f80e3738632
```

Do one of the two, not both. If you used the GUI, your file has the `value` and `unit` lines but no `from.source` and `from.note` lines. The value overrides the catalog value of 920 g, and `em_test_2026_10` is a source that exists in the demo (`fune source ls` lists them).

## 6. See the budget and the verdict change

```console
$ fune budget mass
Mass budget
budget
     Row             Current   With growth   Allocated   Margin
────────────────────────────────────────────────────────────────
 ✔   AOCS            1.04 kg       1.10 kg     1.60 kg    502 g
 ✔   C&DH             90.0 g        99.0 g       200 g    101 g
 ✔   Communication     340 g         370 g       500 g    130 g
 ✖   Payload         1.20 kg       1.44 kg     1.20 kg   -240 g
 ✔   Power           1.45 kg       1.58 kg     2.00 kg    423 g
…
```

Before the change the Payload row read 920 g, 1.10 kg, 1.20 kg and 96.0 g. It is now marked ✖: with growth margin it needs 1.44 kg against an allocation of 1.20 kg. Open the Overview page in the GUI and the same row is red there. The total is still within its limit, because the satellite as a whole has room: the allocation of each subsystem is checked separately from the total.

<!-- screenshot: / as mass-over — the Mass budget panel with the Payload row over its allocation (red), after CAM.mass is set to 1.2 kg -->
*The Payload row over its allocation after the change.*

## 7. Run an analysis

Analyses are run by ID. `fune analyze ls` lists them all. Run the mass-properties analysis of the structure:

```console
$ fune analyze structure.mass_properties
✔ structure.mass_properties ran
     KPI          Value
──────────────────────────
 ✔   Total mass   5.83 kg
 Verdict   Target   Item         Detail
───────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok               Total mass   With growth margin 5.83 kg / usable 12.0 kg (limit 12.0 kg × (1 − system margin 0 %))
Use --json for the full result
```

An analysis prints KPIs (key numbers) and verdicts. A verdict is `ok`, `warning` or `violation`. Here the total mass is far below the 12 kg limit, which comes from the requirement `R_MASS`. Before your change this analysis said 5.55 kg.

The numbers differ between commands because each counts a different thing. The analysis above reports the total with growth margin. The mass budget in step 6 also adds the propellant, so its total is 6.64 kg.

The compliance table shows every requirement against the analysis or budget that verifies it. Here it is trimmed to the R_MASS row and the last line:

```console
$ fune requirement compliance
 Verdict          ID               Requirement                                       Constraint   Cases     Detail
…
 ✔                R_MASS           The satellite total mass shall not exceed 12 kg.   ≤ 12.0 kg   default   Total 6.64 kg against limit 12.0 kg
…
16 requirements: 11 ok, 1 warning, 1 violation, 3 not verified
```

The demo already has one violation (`R_STORAGE`) and one warning (`R_STT_SUN`), so you have verdicts to read. See the [analyses](../analyses/index.md) for what each one means.

To run the same analysis from the GUI, open a subsystem page, for example Structure. Its Overview tab shows the KPIs and verdicts, and the "Analysis results" and "Settings" tabs go deeper.

## 8. Read the change in git

Everything you did is a change to one file:

```console
$ git status --short
 M parts/payload/CAM.toml
$ git diff
diff --git a/parts/payload/CAM.toml b/parts/payload/CAM.toml
index 5d01e09..0001e51 100644
--- a/parts/payload/CAM.toml
+++ b/parts/payload/CAM.toml
@@ -8,6 +8,12 @@ point = "CAM_PORT"
 [pose]
 position = { x = 0.0, y = -60.0, z = -90.0, unit = "mm" }

+[attr.mass]
+value       = 1.2
+unit        = "kg"
+from.source = "em_test_2026_10"
+from.note   = "Heavier lens barrel"
+
 [ports.PWR]
 net = "P5V"
```

This is the whole idea of iwafune: a design change is a readable diff. You can commit it, open a pull request for it, or throw it away.

## 9. Undo

`fune history` lists the edits that can be undone. `fune undo` restores the files that the last edit changed:

```console
$ fune history
   1          fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  1 file(s)
(keeps the last 100 edits; `fune undo` / `fune redo` move along this list)
$ fune undo
  modified parts/payload/CAM.toml
✔ Undid: fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  rev b3:5e54b14f210a
$ git status --short
```

`git status` prints nothing: the working tree is back to the baseline, and the `rev` is the one from step 2 (`b3:5e54b14f2…`). `fune redo` puts the change back. In the GUI, press `Ctrl+Z` (`⌘Z` on a Mac) to undo and `Ctrl+Y` (`⇧⌘Z`) to redo. The GUI and the CLI share one history, and the "Change history" list at the bottom of the sidebar shows the edits of this session.

Undo refuses to run if the files changed some other way since the edit (an editor, `git checkout`), so it never overwrites work it did not make.

## Where to go next

- Build your own project from nothing: [Build your first satellite](first-satellite.md).
- Learn the model behind what you just saw: [Concepts](concepts.md).
- Find the command for a task: [CLI guide](cli.md). Find a page of the GUI: [GUI guide](gui.md).
