# Report export

`fune report export` writes the tables and figures of a design report from the design data and the analyses, as files you include from **your own Typst document**.
You write the prose; iwafune writes the numbers. Nothing in the design changes, and a new export is always the same bytes for the same design (a snapshot test checks it).
This is the first stage of #13: there is no PDF build yet (`fune report build`), no LaTeX/CSV fragments, no `reports/<ID>.toml`, and no GUI buttons.

## Use

```bash
fune report export                                  # all six items, the default case, English, into exports/report/
fune report export --case hot_imaging --case cold_safe --lang ja
fune report export --item mass --item thermal-result --out build/report
typst compile exports/report/example.typ            # a complete example that includes every item
```

| Option | Meaning |
|---|---|
| `--out <dir>` | Output directory. Default: `exports/report` under the project root. Stale `figs/*.svg` from a previous export are removed. |
| `--case <ID>` | Analysis case (`cases/<ID>.toml`) for the case-dependent items; repeat for several. Default: the analyses' default case (`analysis/common.toml` `default_case`), once. |
| `--item <name>` | One of `mass`, `mode-power`, `link-budget`, `thermal-result`, `thermal-history`, `power-history`; repeat for several. Default: all. |
| `--lang ja\|en` | Language of headings, captions and notes (default `en`). Names that come from your design (parts, modes, cases) are as you wrote them. |

The same operation is `POST /api/v1/report/export` with `{ items?, cases?, lang? }`. The server returns the files' contents (`files[].path` and `files[].content`, `revision`, `unavailable`) and writes nothing;
the CLI writes what the same service function returns. So the two produce the same bytes, and the export needs no exception to "design files are written only through Ops" (it writes derived files, not design files).
`exports/` is derived: add it to your `.gitignore` (the demo does) and regenerate it, or commit it if you want the exact tables in review.

## Files

```
exports/report/
  data.json        every exported table, figure summary and citable value (SI values and display strings)
  iwafune.typ      the Typst module (the same file every time; no language-dependent text)
  example.typ      a document that includes everything that was exported
  figs/
    thermal-history-<case>.svg
    power-history-<case>.svg
```

Excerpt of `data.json` (the demo, mass table):

```json
{
  "schema": 1,
  "revision": "b3:f40b…",
  "project": "IWAFUNE 6U (demo)",
  "lang": "en",
  "items": ["mass", "mode-power", "link-budget", "thermal-result", "thermal-history", "power-history"],
  "cases": ["hot_imaging"],
  "tables": [
    {
      "item": "mass",
      "caption": "Mass budget",
      "columns": [
        { "header": "Item", "align": "left" },
        { "header": "Unit mass", "unit": "g", "align": "right" }
      ],
      "rows": [
        { "style": "group", "cells": [{ "text": "AOCS" }] },
        { "style": "body", "cells": [{ "text": "Reaction wheel, 10 mN·m·s class (RW_X)" }, …, { "text": "152", "si": 0.152 }] }
      ],
      "notes": ["Masses include the growth margin set per maturity. …", "Model revision: b3:f40b…"]
    }
  ],
  "figures": [{ "item": "thermal-history", "case": "hot_imaging", "file": "figs/thermal-history-hot_imaging.svg", "…": "…" }],
  "values": [{ "key": "part.RW_X.mass", "text": "152 g", "number": "152", "unit": "g", "si": 0.152 }]
}
```

Row `style` is `body`, `group` (a heading row), `subtotal`, `total` or `margin`. A cell's `si` is the SI number (absent for text); `detail` is a short note shown under the value (a part's state in the power table).
Any table or figure whose inputs are missing is still present, with an `unavailable` text ("Not available: missing inputs (…)") and no rows; the command lists them under "Not available because" and the rest of the export is unaffected.

## From Typst

```typst
#import "report/iwafune.typ": *

= Mass budget
The dry mass is #value("budget.mass.total"), of which the reaction wheels are #value("part.RW_X.mass") each.
#mass-table()

#link-budget(link: "uhf_down@TOKYO_GS")
#thermal-result(case: "hot_imaging")
#thermal-history(case: "hot_imaging")
#power-history(case: "hot_imaging")
```

| Function | Item |
|---|---|
| `#mass-table()` | Items per subsystem with quantity, unit mass, mass, growth and mass with growth; subtotals; total; limit; margin; propellant and wet total when counted |
| `#mode-power-table()` | Components by modes (mean power without growth margin, with the state and time fraction under it); totals per mode without and with growth margin; eclipse totals when they differ; peak |
| `#link-budget(link: auto)` | An input table and a result table per link × station (`link: "<name>@<station>"` picks one). The caption names the link, its direction, the station and the frequency |
| `#thermal-result(case: auto)` | Nodes by the steady-state cases (hot, cold, …) with min, max over the cases, the allowable range (operating, else the first kind) and the node's verdict |
| `#thermal-history(case: auto)` | Figure: all node temperatures on one axis, eclipse bands (from `power.orbit_generation` of the same case), the allowable ranges as dashed lines |
| `#power-history(case: auto)` | Figure: generation and load, and the battery state of charge with its lower limit, with eclipse bands |
| `#value("<key>", unit: true)` | One value with its unit for prose. Keys: `part.<ID>.mass` (per unit), `budget.mass.{total,limit,margin}`, `kpi.<analysis>.<kpi>[@<case>]` |
| `#value-si("<key>")` | The SI number of that value |
| `revision` | The design revision the files were written from (compare with `fune status`) to find stale tables |

`case: auto` returns every exported case. A case that was not exported stops the compilation with a message that says so. Each function returns a `figure` of kind `table` or `image`, so your numbering and `@ref` apply
(captions are above tables and below figures; chapter-style numbering is whatever your document configures). The paths of `data.json` and the figures are resolved from `iwafune.typ`, so the module can be imported from any document
(if the document is not below the module's directory, give `typst compile` a wider `--root`). Tested with Typst 0.15.1; `typst` is not needed to run `fune report export`.

## Conventions the output follows

- **Units in the column headers**, one unit per column (chosen by `sat_units::format`'s display preferences from the smallest value, or from the largest for the power and time columns); numbers to **3 significant figures** (`sat_units::format::format_number`).
  The link tables have a unit column because each row is a different quantity; temperatures are in °C except the system noise temperature (K).
- **Margins**: each table says whether the growth margin is included and how the margin is measured (notes under the table, and the labels of the rows).
- **Case in the caption** for the items that depend on the case, and **the model revision in a note under every table and figure** and in `data.json`.
- Values are the analyses' and the budget's results; nothing is recomputed in the export. Verdicts are the analyses' verdicts.
- Figures are plain SVG: white background, thin axes, monochrome-safe line styles (8 colours and 4 dash patterns, in `fune_report::palette`), text as real `<text>`, fixed 2-decimal coordinates.
  The width is 640 units (about 160 mm at report width).

## Where it lives

| Responsibility | Place |
|---|---|
| Data model, `data.json` shape | `fune_report::model` |
| Headings, captions, notes (ja/en) | `fune_report::labels` |
| SVG chart | `fune_report::{chart, legend, ticks, svg, palette}` |
| Typst module and example | `fune_report::typst`, `crates/app/report/src/template/iwafune.typ` |
| File layout | `fune_report::files` |
| Building the items from analyses | `fune_service::report::{mass, power, link, thermal, history, values}` |
| CLI, HTTP | `fune report export`, `POST /report/export` (`fune_service::report::export`) |
