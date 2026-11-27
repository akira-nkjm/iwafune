# Roadmap toward conceptual design

2026-11-06. We surveyed past conceptual design reports for small satellites (47 of them) and the template of the calculation sheets, and listed in priority order
what conceptual design repeatedly needs but does not exist yet.
Each item is managed as GitHub Issues #1 to #15 (parent #16); the P1 items are closed, and #14 (P2), #15 (P3), #16 (the parent) and #17 (higher-degree gravity) are open.
The counts are "of the 47 design reports, how many dealt with that item".
The analyses that exist now are listed in [analyses/index.md](analyses/index.md), and `fune analyze ls` lists them for your version.

## What the survey showed

- About half of the themes are deep space, planets and Lagrange points, about 10% are the Moon, and about 30% are Earth orbit. Earth-centered orbits alone cannot cover the majority.
- The substance of the design is a calculation-sheet template handed down across academic years (component selection, per-mode power, SAP/BAT, link, disturbances, structure, thermal, surface properties).
  It has problems such as the same part's values disagreeing between tabs and values being transcribed by hand into a tab for the clean copy, and a single source of truth, a mode table and automatic generation of report tables are a direct remedy.
- The formats of the design reports are 30 in LaTeX, 6 in Typst, 5 in Word and 2 in Google Docs. In 2025, 3 of the 4 teams used Typst.
- The points raised repeatedly at the oral examinations (no power analysis during eclipse, unclear whether margins are included, significant figures, table numbering, distinguishing the kinds of attitude requirements)
  can be prevented mechanically by consistency checks in the model and by report templates.

## P1 (the first list; all done except where a "not done" is stated)

| # | Feature | Basis | Status |
|---|---|---|---|
| 1 | ΔV budget and propulsion subsystem (maneuver list, rocket equation, propellant and tanks, propulsion kinds and attributes) | 39 reports | Done: `propulsion.maneuvers`, `propulsion.dv_budget`, `propulsion.propellant`, `propulsion.thruster_usage`, `propulsion.orbit_history`, `propulsion.mass_properties` |
| 2 | Generalized central bodies and analytical transfer ΔV (Moon, planets, L points; Hohmann, plane change, patched conic) | about 60% of themes | Done: central bodies other than the Earth, Hohmann, bi-elliptic and patched-conic transfers, dated (Lambert) transfers, `propulsion.transfer_check`, `propulsion.launch_window`, and halo orbits about L points |
| 3 | `orbit.eclipse`: seasonal variation of the β angle and the longest eclipse over the mission lifetime (determines the worst case for power and thermal) | 31 reports | Done as `orbit.eclipse_season` and `orbit.geometry` |
| 4 | Altitude decay from atmospheric drag and the ΔV for orbit maintenance (by solar activity condition) | 34 reports | Done: `orbit.decay` |
| 5 | Catalog of launch vehicles and dispensers (mass limit, envelope, quasi-static load, stiffness requirement, angular rate at separation) and a compatibility check | 32 reports | Done for the catalog (kinds `launcher` and `dispenser`, selected in `conditions/launch.toml`; `model.md` §5.6) and `structure.launch_fit` (mass, envelope, natural frequencies; the mass limit can be a curve over the departure C3, `payload_mass_by_c3`, when the launcher provides the departure burn of a dated transfer, #375); the loads and minimum frequencies feed `structure.static_load` and `structure.modal_estimate`; the frequency checks of `structure.launch_fit` use the whole-body modes of `structure.modal_fem` (#370). Not done: a launch-site declination limit on the departure asymptote, random vibration (PSD), interface details, and a GUI for choosing the launcher |
| 6 | Extending modes (longest duration, sunlit and eclipse distinction, attitude law), plus the number simultaneously operating and redundancy (cold, warm) | every table in the template has a "simultaneously operating" column | Done: `fune mode timing`, `fune mode set --active/--spare-state`, `fune mode view`, an attitude law per mode and phase |
| 7 | Inverse sizing of power (required SAP area and battery capacity) and support for distance from the Sun | 31 reports | Done: `power.sizing`, with the irradiance scaled by the Sun distance |
| 8 | Extending the link calculation (PFD compliance under the Radio Act, distances determined by the case, deep space stations) | included as standard in the template | Done (#8): the PFD check and the distance window of the orbit in `comm.link_budget` |
| 9 | Entities for the mission, success criteria and phases (mission sequence) | 45 and 31 reports | Phases are done (`phases/`, an orbit and attitude-law override per phase for the cases that name it; `model.md` §7.10). The mission and its success criteria are not (`mission/` is reserved) |
| 10 | Extending requirements (rationale, origin, parent and child, allocation target, verification method) and a requirements compliance table | structure of the subsystem design reports | The compliance table and the criteria in requirements (budget limits, check thresholds and the mission duration held only by requirements) are done; the tree is not |
| 11 | Preparing thermal inputs (temperature ranges in 3 kinds: storage, operating and performance; distinguishing heat dissipation from power consumption; surface materials of faces; margin policy; worst-case presets) | 28 surface property tables and 17 allowable temperature tables | Done (#11): `temp_survival`, `temp_op`, `temp_perf`, heat dissipation from the power minus the RF output, and surface coatings (`model.md`, [analyses/thermal.md](analyses/thermal.md)) |
| 12 | Extending attitude analysis (slew time, disturbances from thrust error, ΔV and fuel for unloading) | 40 reports | Done: `aocs.slew_time`, `aocs.rw_unloading`, `aocs.unload_time` |
| 13 | **Document generation**: insert tables and figures built from the model into a Typst template. For other formats (LaTeX, Word, slides), write out table fragments | below | First stage done: `fune report export` ([report.md](report.md)). Not done, and not planned for now: `fune report build` (PDF), fragments for other formats, `reports/<ID>.toml` and GUI buttons |
| 18 | Look up observed and predicted space weather values (F10.7, Ap) by date (a prerequisite of #4; added 2026-11-06) | needed for the atmospheric drag analysis | Done: `fune data fetch space_weather`, used by `orbit.decay` |

## P2 (raise the quality of conceptual design)

Mass properties per configuration state (done for stowed and deployed placements: `structure.mass_properties` follows the configuration; remaining propellant: `propulsion.mass_properties`) / random vibration and honeycomb panel strength /
ground tracks, coverage and revisit time / stray light checks in 3D (field-of-view occlusion and plume clearance are done: `aocs.sensor_fov`, `structure.cone_clearance`; orbital lifetime and deorbit are done: `orbit.decay`) / an entity for trade-off tables /
extending the catalog (TRL, heritage, price, lifetime) and a materials catalog / parametric initial sizing /
turning the conceptual design checklist into diagnostic rules / receiving external payload analyses through fune-tool/1 / aggregating part prices /
higher-degree gravity field (beyond degree 70; the coefficients are distributed with a release, as with DE440. #17, added 2026-11-06)

## P3 (later)

orbit optimization / radiation single-event effects (the total ionising dose is done: `orbit.trapped_flux`, `orbit.trapped_dose`, `orbit.total_dose`; CR3BP halo orbits are done too) / formation flying / error budgeting / FMEA, cost, schedule, regulations, RTG

## Policy for document generation

The first stage follows this policy; the parts marked "not made" are the later stages.

- **Make Typst the main route.** It is starting to be used by the people who write design reports. `fune report export` writes `exports/report/data.json` and `figs/*.svg` and a Typst module; people write the body in `.typ` and insert the generated tables ([report.md](report.md)).
  - Not made: `fune report build <ID>` (building a PDF with Typst in the same process, with the Rust `typst` crate).
  - The module provides `#mass-table()`, `#mode-power-table()`, `#link-budget()`, `#thermal-result()` and more.
  - The export unifies significant figures, units, whether margins are included and the layout of the tables. The model's revision is shown in the table footnote so that stale tables do not remain.
- **For other formats, write out fragments** (LaTeX booktabs, CSV, docx tables, PNG/SVG). Not made. Every table in the GUI would get "Copy as Typst / LaTeX / CSV / image" (the same feature as the CLI).
- **Put the document definition in the model** (`reports/<ID>.toml`: document number, version, revision history, the tables and figures to include, the template). Not made. Presets would be provided for the conceptual design report, PDR and subsystem design reports.
- Drawings (CAD) are out of scope. From iwafune we hand over the BOM, equipment per face, and center of mass and inertia as CSV, and in the future we will write out STEP / glTF.

## Items still missing from the data model (candidates to add to model.md)

Mission (purpose, significance, comparison with precursor missions) and success criteria, requirement rationale and the requirement tree,
a materials catalog, trade-offs as an entity, catalog TRL, heritage and price,
operational constraints of ground stations, data requirements, document metadata, references (generated from the source ledger).

## App features planned (not yet made into Issues)

- Build end-to-end tests (Playwright) for the main GUI edits and analyses, and add them to CI.
- File watching and notification to the GUI (so that the GUI follows edits made in an outside editor or the CLI right away).
- A page showing the difference from the base branch (changes in values and budgets).
- A view that lines up trade candidates (`candidates`) for comparison.
- Handoff to and from external analysis tools in general (input and result formats, how to re-run).
- A simulation that follows the initial operations after separation over time (power on → deployment → detumble → sun acquisition).
