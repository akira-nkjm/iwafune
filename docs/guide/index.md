# iwafune user guide

iwafune is a workbench for the conceptual design of small satellites. You keep the design as a directory of plain TOML files in git. You edit it with the `fune` command-line tool or in a web GUI, and you run subsystem analyses on the same data.

This guide is for people who are new to iwafune. It shows how to install the tool, build a design, run analyses and read their verdicts, and it points to the reference pages when you need the details.

## Who it is for

- Spacecraft systems engineers who need mass, power and data budgets, and quick subsystem checks, while the design is still changing.
- Students and project teams who write a conceptual design report and want the numbers in the report to come from one place.
- Anyone who wants a design review to work like a code review: branches, pull requests, readable diffs.

You do not need to know Rust or TypeScript. You do need a terminal, git, and a basic knowledge of spacecraft subsystems.

## Three principles

**Design data lives in git.** A project is a directory with a `fune.toml` file. Parts, modes, requirements, orbit settings and analysis cases are small TOML files. A change to one value is a one-line diff. Every value can carry a unit, a source and a note, so a reviewer can see where a number came from.

**The CLI and the GUI are equals.** Both make the same edits through the same code. Every edit you make in the GUI can be shown as a `fune` command, and the other way round. The files they write are byte-for-byte identical. Use whichever suits the task, and mix them freely.

**Analyses return verdicts.** An analysis builds its inputs from the design and returns key numbers and a verdict: OK, warning or violation. If the design lacks an input, the analysis does not guess. It stops and lists what is missing, with a way to fill each item.

## What iwafune does not do

- It is a conceptual-design tool. The analyses are simple, typed models for early trade-offs. They do not replace detailed tools such as CAD, thermal solvers or full finite-element packages.
- It ships no data of real parts. The demo project is fictional. You enter values from your own datasheets and tests, and record the source of each.
- It does not build a PDF report. `fune report export` writes tables and figures that you include from your own Typst document (see [report export](../report.md)).
- It has no mission-objective or success-criteria entity, and requirements are a flat list, not a tree. See the [roadmap](../roadmap.md).
- The GUI is a local web server for one project. People collaborate through git, not through a shared GUI server.

## A map of the documentation

**Start here.** Read these in order if you are new.

| Page | Read it to |
|---|---|
| [Install](install.md) | Install `fune`, check that it works, update it and fetch the optional data sets |
| [Quickstart](quickstart.md) | Take a short tour of the demo satellite: check it, edit one value, see the verdict change, undo |
| [Build your first satellite](first-satellite.md) | Create a project from nothing and get a first budget and a first verdict |

**Using iwafune.** Learn how the tool thinks, then look up how to do a task.

| Page | Read it to |
|---|---|
| [Concepts](concepts.md) | Learn the mental model: parts, catalog, modes, cases, requirements, Ops |
| [CLI guide](cli.md) | Find the command for a task, with examples and a command reference |
| [GUI guide](gui.md) | Go through the GUI page by page |

**Analyses.** One page per area. The [analyses overview](../analyses/index.md) explains how analyses, checks and verdicts fit together and lists every analysis.

| Page | Covers |
|---|---|
| [AOCS](../analyses/aocs.md) | Sensor fields of view, slew time, wheel saturation and unloading |
| [Budgets](../analyses/budgets.md) | Mass, power and data budgets, allocations and margins |
| [Communications](../analyses/comm.md) | Link budget, ground station visibility, downlink capacity |
| [Orbit](../analyses/orbit.md) | Beta angle, eclipses, drag decay, mission geometry |
| [Power](../analyses/power.md) | Mode loads, generation, energy balance, battery depth of discharge, sizing |
| [Propulsion](../analyses/propulsion.md) | Maneuvers, delta-v budget, propellant and tanks |
| [Radiation](../analyses/radiation.md) | Ionising dose from trapped protons and electrons behind a shield, against the dose a part tolerates |
| [Structure](../analyses/structure.md) | Mass properties, clearances, launch loads, natural frequencies, launcher constraints |
| [Thermal](../analyses/thermal.md) | Radiator and heater sizing, temperature limits, the swing through sunlight and eclipse |

**Reference.** Look things up here.

| Page | Read it to |
|---|---|
| [Data model](../model.md) | Look up every file format and key |
| [Report export](../report.md) | Export tables and figures for a Typst document |
| [Trapped radiation data](../trapped-radiation.md) | See the models, sources and attribution of the radiation data |

**Internals.** For people who want to understand or change the program.

| Page | Read it to |
|---|---|
| [Architecture overview](../architecture-overview.md) | See how the program is put together, in plain terms |
| [Development](../development.md) | Build from source and work on the repository |
| [Roadmap](../roadmap.md) | See what is planned |

## Conventions in this guide

- In output blocks, a line that starts with `$` is a command you type. Lines without `$` are what the program printed.
- A line with only `…` means that long output was trimmed. Home and project paths are shortened to `~/…` or `/path/to/…`.
- `my-sat/` and `my-demo/` are example directories. Use any name you like.
- Screenshots are marked in the source by an HTML comment. The documentation site replaces each one with an image.
