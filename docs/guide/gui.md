# GUI guide

The GUI is a web application that `fune` serves from your project directory. It reads and writes the same files as the CLI, so you can edit in either and see the result in both. This page goes through it page by page, in the order of the navigation on the left. For each page you get what it shows, what you can edit and a typical task. The patterns that repeat on many pages are explained once, in a section before the pages.

Start it from inside a project:

```bash
fune gui            # http://127.0.0.1:3000, or the next free port; opens the browser
fune gui --no-open  # do not open the browser
```

The GUI has English and Japanese dictionaries. This guide uses the English labels; the Japanese edition adds the Japanese label in brackets.

Contents:

- [The window](#the-window)
- [Patterns used on many pages](#patterns-used-on-many-pages)
- [Overview](#overview)
- [Design](#design): Components, Catalog, Layout, System diagram, Modes, Requirements
- [Subsystems](#subsystems)
- [Conditions](#conditions): Orbit & stations, Analysis cases, Phases
- [Project](#project): Diagnostics, Settings, Units & definitions, Sources
- [Typical tasks and where to do them](#typical-tasks-and-where-to-do-them)

## The window

<!-- screenshot: / — the whole window: the sidebar with its sections on the left and the Overview page on the right -->
*The window: the sidebar and a page.*

The **sidebar** has the project name, a search box, the navigation and, at the bottom, the project's git branch and revision, the diagnostics counters, the "Change history" row, the language row and the appearance row. The navigation has five sections:

| Section | Pages |
|---|---|
| (top) | Overview |
| Design | Components, Catalog, Layout, System diagram, Modes, Requirements |
| Subsystems | One page per subsystem |
| Conditions | Orbit & stations, Analysis cases, Phases |
| Project | Diagnostics, Settings, Units & definitions, Sources |

What you should see after `fune gui`: the Overview page, with the counts of parts, modes, nets and cases under the title.

- **Search.** Click the search box or press `Ctrl+K` (`⌘K` on a Mac). Type a few letters and choose a page, a part, a net or a mode to go there.
- **Sidebar.** `Ctrl+B` (`⌘B`) opens and closes the sidebar. When it is closed, a narrow rail with icons remains.
- **Diagnostics counters** (errors and warnings, under the branch name). They are the same counts as `fune check`. Click them to open the Diagnostics page.
- **Change history.** The row opens a popover with the changes you made in this session (since the page was loaded), newest first. Each entry shows the command that makes the same edit, with a button to copy it. Undo and redo steps appear in it too. The header of the popover has **Undo** and **Redo** buttons, which do the same as `Ctrl+Z` and `Ctrl+Y` (see [Undo and redo](#undo-and-redo)).
- **Language.** The "Language" row switches between Japanese and English. The choice is remembered in your browser.
- **Appearance.** The "Appearance" row chooses "Follow the OS", "Light" or "Dark".

If the GUI cannot reach the server, a red "Cannot reach the server" box appears under the counters. Check that `fune gui` is still running.

## Patterns used on many pages

### Edit dialogs and the "same operation as a command" preview

An edit that needs more than one value opens a dialog: add a part, add a mode, add a requirement, rename something. A dialog is anchored near the top of the window. When it is taller than the window, it scrolls inside, and the buttons stay reachable. Press `Enter` to confirm; this works only while the dialog has no error, the same as pressing the confirm button, which is disabled until the input is valid.

Before you confirm, the dialog shows a box titled **Same operation as a command**. It holds:

- the `fune` command (or commands) that make the same edit, with a **Copy command** button;
- a line "Files changed:" that you can open to read the diff of each file, with the labels new, modified, deleted or renamed;
- warnings, and the reason if the edit would be refused.

The preview comes from a dry run, so nothing is written until you press the confirm button. Use it to learn the CLI, or to copy a command into a script. If the edit would leave a field of an existing record out, for example when you replace a case, the preview says "Removed from <file>: <keys> (not in the new record)".

Small edits do not use a dialog. Where a value is shown with a pencil on hover, click it, type, and press Enter to save, or Escape to cancel. A value is typed as text, with its unit (for example `1.2 kg`), and the service reads it exactly as `fune set` does ([value syntax](cli.md#value-syntax)). An error is shown under the field, and nothing is written.

When an edit is saved, a short message names the files that changed. If the design changed elsewhere in the meantime (a file edited, a CLI command run), the message says "The design changed elsewhere", the GUI reloads the latest design and you try again.

### Removing something that is still in use

You can remove a case, a phase, a mode, a configuration, a link, a ground station or a catalog item from its page. If something still refers to it, the removal is refused. The dialog preview then shows the reason as a message that lists what refers to it, for example "Analysis case hot_imaging cannot be removed: referred to by `default_case` in analysis/common.toml; `quantify.cases` of requirement(s) R_STORAGE. Change those first". Change those first, then remove. The CLI gives the same message ([Cases](cli.md#cases-phases-and-configurations)). A removal that goes through can be undone.

### Missing-input rows and their fix actions

When an analysis lacks inputs, the subsystem page lists them under **Missing inputs (n)**. Each row names the missing item and the reason. Where the tool knows how to fill it, the row has a link-like action:

| Action | What it opens |
|---|---|
| "Enter value" | A dialog to type the value. If the part uses a catalog item, you choose where to write it: "Override on this part", or the catalog item (which applies to every part that uses it) |
| "Set on a part" | Choose a part and type the value; it is saved as an override on that part |
| "Assign an existing part" | Choose a part that should take a role; its kind changes to one with the role |
| "Add a part" | The Add component dialog, pre-set for the missing kind |
| "Pick a part", "Pick a catalog item", "Pick a link", "Pick a configuration", "Pick a maneuver" | A list of the choices in your design |
| "Add a mode", "Add a maneuver", "Add a ground station", "Add a comm link", "Add an analysis case" | The same dialog as the Add button of that page |

They are the same fixes that `fune analyze` prints as commands ([tutorial, step 11](first-satellite.md#11-run-an-analysis-and-fix-the-missing-inputs)). Every dialog shows the command preview. Missing inputs also appear in the Overview budgets and on the Requirements page ("Inputs are missing").

<!-- screenshot: /systems/power — the Missing inputs list of an analysis with the "Enter value" and "Add a part" actions visible -->
*A list of missing inputs with fix actions.*

### The case selector

Every subsystem page has a selector at the top right with the label "Analysis case". Its first entry means "Automatic (default where needed)": analyses that need a case use the default case, and the others look at all modes. When the default case decides, the entry says "Default for analyses requiring a condition: <case>". Choose a case to run every analysis of the page for it. The **Recompute** button next to it runs the analyses again.

The choice is kept in the page address (`?case=<ID>`), so you can share a link. If a link names a case that no longer exists (you removed or renamed it, or you undid its creation), the page shows the default selection and a notice: "Analysis case <ID> no longer exists; showing the default selection."

### The configuration selector

The Layout page has a selector labelled "Placement configuration", with "Base" as the first entry. It chooses which configuration of the part poses you see and edit (see [Concepts](concepts.md#configurations)). When you move a part with a configuration chosen, the pose is written to that configuration only. "Manage configurations" adds, edits, renames and removes configurations. The choice is kept in the address (`?configuration=<ID>`).

### Undo and redo

`Ctrl+Z` (`⌘Z`) undoes the last edit, and `Ctrl+Y` (`⇧⌘Z`) redoes it. Both work anywhere in the window, except in a text field, where they undo the typing. You can also press the **Undo** and **Redo** buttons in the header of the "Change history" popover in the sidebar, or the buttons in the toolbar of the System diagram page.

The history is the one `fune undo` and `fune redo` use, so you can undo a CLI edit from the GUI and the other way round. The list in the popover shows what was done in this browser tab; it starts empty when you reload the page, while the undo history itself is kept in the project (`.fune/history.json`). An undo is refused if the files changed some other way since the edit.

### Tables and trees

Table headers sort: click once for ascending, again for descending, a third time for the original order. Trees have "Expand all" and "Collapse all". Search boxes filter what you see; they do not change the design.

### The 3D view

The Layout page and the "Thermal image" tab of the Thermal page show the satellite in 3D.

| To | Do |
|---|---|
| Orbit (rotate) | Drag with the left mouse button, or one finger. You can roll about any axis, upside down too |
| Pan | Mouse: middle or right button drag, or Shift and left drag. Trackpad: two-finger scroll |
| Zoom | Mouse wheel, or pinch on a trackpad. It zooms about the pointer. The zoom-out and zoom-in buttons at the top right zoom about the centre |
| Look from a direction | Click a face, an edge or a corner of the **view cube** at the bottom right. The view moves to the nearest upright view, with no tilt. A corner gives a Y-shaped view along the diagonal of the cube |
| Roll the view | The two curved arrows beside the view cube (counter-clockwise and clockwise) turn the picture about the viewing line: by 120° when you look from a corner, otherwise by 90° |
| Reset | The **Home** button (a house) at the top right resets the camera and the direction |
| Help | The question-mark button at the top right lists these controls |

<!-- screenshot: /design/layout — the 3D view with the view cube and the two roll arrows at the bottom right and the zoom, Home and help buttons at the top right -->
*The 3D view controls.*

### The 2D diagram view

The System diagram page uses the same ideas in 2D. Zoom with the wheel or a pinch, pan with a two-finger scroll or by dragging the background or the middle button. The keys `+`, `-`, `0` (fit to view), `1` (100 %) and `F` (zoom to the selected item) work when no text field has focus.

## Overview

**What it shows.** The Overview page is the dashboard of the project. The subtitle gives the name and the counts of parts, modes, nets and cases.

- **KPI cards.** Headline numbers from the analyses, such as "Mass margin". Each card has a value, a fill bar, a verdict colour and a line of related numbers. Click a card to open the subsystem page behind it. A card that has nothing to show for your design does not appear.
- **Conditions.** A second row of cards for the orbit and environment.
- **Budget panels**: one per budget kind (mass, power, data). A panel has the budget title, the mode or case, the margin, the limit and the system margin, a row per subsystem with "Current", "With growth", the allocation and the margin, and the total. In the mass budget, "Dry" and "Wet" totals and the propellant row appear when the design has a propulsion system. "Limit from requirement R_… (edit it there)" says where the limit lives. Rows of parts that have no value are listed as not counted. Missing inputs of a budget are listed too.
- **Diagnostics.** The counts of errors, warnings and info, with "See all".

**What you can edit.** Nothing on this page. Cards and rows are links to the page where you can edit.

**Typical task.** After an edit, come back here to see whether a budget turned red.

<!-- screenshot: / — Overview with KPI cards and the mass and power budget panels -->
*The Overview page.*

## Design

### Components

**What it shows.** Components lists the parts. It has three tabs:

- **Overview**: cards grouped by subsystem, group and assembly.
- **Tree**: the product tree, with the directory structure of `parts/`.
- **Table**: one row per part with ID, system, kind, catalog item, quantity, mass, power by state, unset attributes and diagnostics. It sorts.

Use "Search parts" (ID, kind or catalog item) and the system filter to narrow the list. Select a part to open its detail, which shows everything about it across subsystems:

| Section | What it shows and lets you edit |
|---|---|
| Basics | Kind, catalog item, the selected candidate among "Trade candidates", quantity, redundancy, maturity, system and location |
| Attribute tables | One table per subsystem (Mass, AOCS, Power, Thermal, …) with each value, where it comes from (catalog, override, source), and the reset button that removes an override and brings back the catalog value. Click a value to edit it. **Add parameter** adds an attribute, with source, location in the source and note |
| Placement | "Mounted on", "Mounting part", "Mount point", the offset and the orientation (angles about X, Y, Z), the derived body-frame pose, and pose overrides by configuration |
| Connections | The ports and the net each one is on |
| State per mode | The state of the part in each mode, with a link to the mode table |
| Harnesses | The cable segments that touch the part |

**What you can edit.** Values (click them), placement, and the parts themselves. **Add** opens the Add component dialog: choose a system or assembly, then "From catalog" (search the catalog and pick an item) or "Kind only", and give an ID. The menu of a part has "Rename", "Group…" and "Delete". Rename rewrites every reference in one step.

**Typical task.** Add a part from the catalog, then set its mass override: Add, choose the item, give an ID, confirm; then open the part and click the mass value.

<!-- screenshot: /design/components/CAM — the detail of the camera part with the attribute tables and the Placement section -->
*The detail of a part.*

### Catalog

**What it shows.** Catalog lists the catalog items of every catalog: your own (`local/…`) and shared ones. Tabs "Tree" and "Table". Filters: catalog, kind and a search box. "Group by" arranges the tree by catalog, manufacturer, kind, maturity, tag or subsystem. Select an item to see "Basics" (name, English name, manufacturer, part number, maturity, tags, datasheet), "Values", "Ports", "States" and the list "Used by" (the parts that use it).

**What you can edit.** Values, and rename or delete the item (deleting is refused while a part uses it). "Add to design" makes a part from it. **New catalog item** creates one, optionally copying the values of an existing part. Items of a shared catalog are read-only: "To change a value, open a PR on the shared repository or override it on the part." The **Shared catalogs** button opens a dialog that lists each shared catalog with its state:

- **Fetch** downloads the commits recorded in `catalog.lock`. It is the same as `fune catalog fetch`.
- **Update** moves the record to the latest commit and shows the items that were added, removed or changed.
- **Add shared catalog** asks for a name, a git URL and a revision. It adds the entry to `fune.toml`; press Fetch afterwards.

After a fetch or an update, commit `catalog.lock`. Only one catalog dialog is shown at a time.

**Typical task.** Describe a new component once: New catalog item, then fill the values with their sources, then use "Add to design".

### Layout

**What it shows.** Layout is the 3D editor of the placement. The header says how many parts are placed and how many are not. A checkered ball marks the centre of mass.

- **Parts tree** (left): the product tree. Use the eye buttons to hide or show a part, and "Show only <part>" to isolate one.
- **The scene**: select a box by clicking it. The tool "Operation" chooses "Move" or "Rotate" handles in the scene; "Snap" rounds the movement; "Structure opacity" makes the structure transparent so you can see inside.
- **Helpers**: toggles for "FoV" (field-of-view cones), "Origin axes", "Grid", "View cube", "Sun" and "Wireframe".
- **Axes, normals, boresights**: coloured lines show spin axes, magnetorquer axes, normals, boresights and thrust directions.
- **Findings**: "Show analysis findings as colours" paints overlaps and plume or field-of-view hits found by the structure analyses. The list "Findings" names them and lets you select the part. A part with no size is drawn as a diamond marker.
- **Placement configuration** (top): see the configuration selector above. The view controls are in [The 3D view](#the-3d-view).

**What you can edit.** The panel "Placement of <ID>" lets you change the mount, the position and the orientation numerically. You can also drag a part with the move or rotate handles. With a configuration chosen, the change goes to that configuration.

**Typical task.** Place a new part. Add it in Components, then select its box in Layout and set "Mounted on", the mount point and the offset.

### System diagram

**What it shows.** System diagram draws the parts as cards and the nets as lines: power and signal connections, with the power systems coloured by rail (battery, solar array, unregulated bus, regulated rails). Groups of parts are drawn as containers.

- **Toolbar.** "Visible subsystems" hides subsystems (the structure is hidden by default). "Layers" turns "Power", "Signal" and "GND" on and off. "Group identical" shows sibling parts with the same catalog item and connections as one card "×N".
- **Details** panel: select a card or a line to see its details. A net has "Rail voltage", "Current limit", "Protocol" and the list of its ports, with "Disconnect" and "Delete".
- **Zoom** buttons and keys, an overview map, and the **Undo** and **Redo** buttons.

**What you can edit.** Positions, routes and connections. Drag a card or a group to move it. Drag from a port to another port to connect them; if neither is on a net, a dialog creates a net. Drag the end of a line to attach it elsewhere on the card edge. Drag a line segment to route it by hand; double-click a segment to add a bend, and a bend to remove it. "Reset positions" returns moved cards to the automatic layout and keeps hand-drawn wires. "Reset positions and wires" clears both.

**Typical task.** Connect a new part to a power rail: drag from its power port to the port of the rail.

<!-- screenshot: /design/diagram — the system diagram with the toolbar, power-coloured lines and the Details panel -->
*The System diagram.*

### Modes

**What it shows.** Modes is the mode table. Each column is a mode, each row a part. The title row of the matrix reads "Component × mode". Switch between "Sunlit" and "Eclipse" in "Lighting period" to see the eclipse override of the states. Power levels per unit, the total power with growth margin, the "Available" power and the "Margin (after system reserve)" appear below the matrix. A panel lists the named mode sequences that the energy balance repeats; "Use for analyses" selects one.

**What you can edit.**

- **Switch a state.** Click a cell to switch the part on and off. Right-click it, or press `D`, to choose another state and the duty (the fraction of each orbit spent in the other states, from 0 to 1). The legend is "● On", "◐ Duty", "— Off".
- **Mode columns.** "Add mode" creates a mode (it can copy another one). The menu of a column renames, deletes and moves it left or right. A mode can have a "Maximum duration" and a "Timing" for the longest-duration evaluation.
- **Saving.** Changes to cells are collected as a draft. The bar at the bottom says "Save the mode table (n cells)" and shows the same command preview as a dialog. Totals and margins already include the draft ("not saved yet"). Press save to write it, or discard it.

**Typical task.** Put the camera in standby during the downlink: right-click its cell in the DOWNLINK column, choose the state, save.

### Requirements

**What it shows.** Requirements has two tabs:

- **Compliance**: one row per requirement with the requirement text, the constraint, the cases, the "Verified by" analysis or budget, the result and the verdict. At the top, counts: "Requirements", "Met", "Warning", "Violated" and "Not verified". A row that is not verified says why: "No analysis check or budget is linked", "Inputs are missing", "The analysis failed", and so on.
- **Requirements**: the list, with "Add requirement", "Edit" and "Remove".

**What you can edit.** Requirements. The dialog takes a short name, the statement (and an English statement), the dimension of the constrained quantity, the source, the direction of the constraint and the value, the cases it must hold in, and either a verifying analysis with its check key or a verifying budget. Typed quantities are read exactly as in `fune requirement add`.

**Typical task.** Add a mass limit: Add requirement, quantity Mass, "at most", the value, verifying budget "mass". The mass budget on the Overview page then shows that limit.

## Subsystems

**What it shows.** There is one page per subsystem: Power, Communication, Thermal, AOCS, C&DH, Propulsion, Structure and Payload (the Payload has no analyses yet). The names follow your project's vocabulary. A subsystem page has the case selector and the **Recompute** button at the top right, and tabs below. Which tabs a page has depends on the subsystem:

| Tab | Where | What it shows |
|---|---|---|
| Overview | All | A table "Analyses", one row per analysis: its kind, the main results, the verdict and the number of missing inputs. Click a row to open all its KPIs, a chart, the verdict texts and the missing inputs. On the Thermal page the Overview shows the chart of node temperatures and ranges instead (storage, operating and performance bands, steady-state dots and transient bars) |
| Maneuvers | Propulsion | The maneuvers in execution order, with Add, Edit and Remove. See below |
| Thermal image | Thermal | Parts coloured by the temperature of their node in 3D, with a source choice (worst across all cases, a steady case, or a transient at a chosen time), playback and a colour scale. An orbit diagram shows the Sun side |
| Couplings and surfaces | Thermal | The conduction and radiation couplings between nodes, the outer-surface coatings and the coatings table. Typing a value writes an override. The effect on node temperatures is shown before you save |
| Analysis results | Thermal | One card per analysis with its verdicts, values and tables (the other subsystems show these in the Overview) |
| Modes | Power, C&DH, Thermal, AOCS | The per-mode values of the subsystem, sunlit and eclipse (the GUI's mode view; `fune mode view <system>`). Power also shows "Where sunlit power goes" and eclipse power from the battery. AOCS shows the attitude law of each mode and the devices |
| Time series | All | Graphs of the time-series analyses. They appear when such an analysis has run |
| Settings | All | The settings of the subsystem, with the file they are in. Values are editable in place. Below the table, a collapsible block per analysis shows the input that was passed to it |

Each analysis is described in the [analysis pages](../analyses/index.md).

**What you can edit.** Settings in place, and the fixes for missing inputs (see above). Two pages have more:

- The **Communication** page has an "Add a link" button and a list "Comm links". Add a link asks for a direction, a transceiver part and a band that a ground station has a performance for. Each link in the list has an edit and a remove button. Editing can rename the link: the downlink choice and the requirement checks that name it are rewritten. Removing is refused while a requirement check names the link. Losses and the modulation are written in the Settings tab.
- The **Structure** page lists "Geometry findings": box overlaps and parts in a cone. You can "Acknowledge" a finding with a reason, which stops it being a warning but keeps it in the list, and "Withdraw" the acknowledgement.

**Typical task.** Fill the missing inputs of the energy balance on the Power page, using the row actions, until the row shows a verdict. The [tutorial](first-satellite.md#11-run-an-analysis-and-fix-the-missing-inputs) does the same with the CLI.

<!-- screenshot: /systems/power — the Power page with the case selector, the Recompute button, the tabs and the Analyses table -->
*A subsystem page.*

<!-- screenshot: /systems/thermal/heat — the Thermal image tab: parts coloured by temperature in 3D, with the temperature source and the colour scale -->
*The Thermal image tab.*

### Maneuvers (Propulsion)

The Maneuvers tab lists each maneuver with its category, method, thruster and target. **Add a maneuver** opens a dialog with the same fields as `fune maneuver add`. Maneuvers keep an order: removing one re-links the next to the one before it.

## Conditions

### Orbit & stations

**What it shows.** The settings under `conditions/` (orbit, ground stations, period) as a table, and the analyses that depend on them:

- **Beta angle and eclipse seasons**: a chart over the year.
- **Altitude history**: the decay of the orbit by drag, with presets for solar activity and, if the space-weather table is fetched, dated values. If the table is missing, the page says to fetch it in Settings.
- **Visibility per ground station**.

**What you can edit.**

- "Create this case" turns the longest-eclipse date or the minimum-beta date into an analysis case (it asks for a case ID and an optional mode).
- **Add ground station** asks for an ID, a latitude and a longitude with units, and optionally the altitude and the minimum elevation. A latitude outside -90 to 90 deg, or a minimum elevation outside 0 to 90 deg, is refused. Each station can be renamed or removed; removal is refused while the communication analysis lists it. Per-band performance is written in the settings.
- Orbit values are edited in the settings table, in place.

**Typical task.** Add the ground station that you will use for the link budget, then check its visibility.

### Analysis cases

**What it shows.** Analysis cases lists the cases of `cases/` with name, description, modes, phase, start time and file. In each row, two icon buttons rename and remove the case (their tooltips are "Rename case <ID>" and "Remove case <ID>"), and a menu button, "Open with this case", lists the subsystem pages: choose one to open it already set to that case.

**What you can edit.**

- **Add a case**: ID, display name, description, mode, phase, uncertainty side, thermal environment, beta angle (with a unit, for example `60 deg`), number of orbits, Sun distance and start time. Adding a case with an existing ID replaces it; the preview lists the fields that would be removed.
- **Choose phase** and **Enter start time** edit one field of a case.
- **Cases from the worst days** creates three cases from the mission geometry of a phase's orbit.
- Rename rewrites the default case and the cases named in requirements. Remove is refused while something refers to the case, and the message says what.

**Typical task.** Add a case for the worst eclipse, then use "Open with this case" to see the Power page for it.

### Phases

**What it shows.** Phases lists `phases/` in order, with name, order, "Starts when", "Ends when", duration, modes used, the overrides and the cases that name the phase.

**What you can edit.** **Add phase**, **Edit** and **Remove** work as elsewhere; removing is refused while a case names the phase. **Edit overrides** opens a dialog for the orbit override (copy the design orbit, then edit values one by one) and the attitude-law override of each mode. The overrides apply only to cases that name the phase.

**Typical task.** Give the disposal phase a lower orbit: Edit overrides, copy the design orbit, change the altitude, then name the phase in a case.

## Project

### Diagnostics

**What it shows.** Every diagnostic of the design with its severity, code, message and file, and a filter "Filter by severity". It is the same result as `fune check`.

**What you can edit.** Nothing here. Fix an error on the page of the thing it names, or in the file.

**Typical task.** Open it when the counters in the sidebar turn red, and work through the errors first.

### Settings

**What it shows.** Settings has three parts:

- **Project**: "Name", "Location", "Branch", "Revision" and "Schema version".
- **External data**: the optional data sets (the DE440 ephemeris, the space-weather table, the radiation data). Each row shows the ID of the data set, its state ("Installed", "Cached" or "Missing"), its size, when it was fetched, what uses it and its licence or attribution.
- **Settings tables**, grouped "Conditions", "Analysis" and "Budget", each with the columns Key, Value and File.

**What you can edit.** A value in a settings table is edited in place; a key with "Not set" accepts a typed value. The **Fetch** button of a data set downloads it, the same as `fune data fetch <ID>` (see [Install](install.md#optional-data-sets)). An error, such as a missing `gh` or `curl`, is shown under the row.

**Typical task.** Fetch the space-weather table before you run the orbit lifetime analysis with dated values.

<!-- screenshot: /project/settings — the Project panel, the External data panel with Fetch buttons, and the settings tables -->
*The Settings page.*

### Units & definitions

**What it shows.** Three tabs: **Units** (every unit with its dimension and SI factor, and "Try a unit", which parses what you type and shows the dimension and the SI value), **Attributes** (the attribute definitions, built-in and your own) and **Kinds** (the kind definitions).

**What you can edit.** "Add attribute" and "Add kind" define your own, and you can rename them. Built-in definitions cannot be renamed or removed.

**Typical task.** Check that a unit you want to type, such as `mN*m*s`, is known, and what dimension it has.

### Sources

**What it shows.** The register `sources/`: ID, kind, name, reference, revision, retrieval date, SHA-256, distribution, date and author. "Open" follows a reference.

**What you can edit.** **Add source** and **Edit** write a source. Sources of a shared catalog are read-only ("Shared"). To remove a source, use `fune source rm`, which is refused while a value cites it.

**Typical task.** Register the datasheet before you type the values from it, so each value can cite it.

## Typical tasks and where to do them

| Task | Where | CLI equivalent |
|---|---|---|
| See whether the design is healthy | Overview, Diagnostics | `fune status`, `fune check` |
| Add a part from the catalog | Components, Add | `fune part add <ID> --catalog <ITEM>` |
| Change a value | Components, a part, click the value | `fune set <PART>.<ATTR> "<value>"` |
| Put a part in a mode | Modes, click the cell | `fune mode set <MODE> <PART> <STATE>` |
| Place a part | Layout, or Components, Placement | `fune part place <ID> --on <PART> …` |
| Connect two ports | System diagram, drag | `fune net connect <PART>.<PORT> <NET>` |
| Read a budget | Overview | `fune budget mass\|power\|data` |
| Run an analysis for a case | A subsystem page, the case selector | `fune analyze <ID> --case <CASE>` |
| Fill a missing input | A subsystem page, the row action | `fune set …` (shown in the dialog) |
| Add a requirement | Requirements | `fune requirement add …` |
| Add or edit a comm link | Communication, Comm links | `fune link add …`, `fune set analysis.comm.link.<name>.<field> …` |
| Undo a mistake | `Ctrl+Z`, or Change history | `fune undo` |
| Fetch external data | Settings, External data | `fune data fetch <ID>` |
| Add a shared catalog | Catalog, Shared catalogs | `fune catalog source add …` and `fune catalog fetch` |

The GUI saves to the files at once; it never holds hidden state. To keep a change, commit it with git.
