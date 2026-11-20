// iwafune report module. Written by `fune report export`; do not edit by hand (it is rewritten on every export).
//
//   #import "iwafune.typ": *
//   #mass-table()                                  // mass budget table
//   #mode-power-table()                            // power by mode table
//   #link-budget()                                 // input and result tables of every link
//   #link-budget(link: "uhf_down@TOKYO_GS")        // one link only
//   #thermal-result(case: "hot_imaging")           // node temperature table
//   #thermal-history(case: "hot_imaging")          // node temperatures over time (figure)
//   #power-history(case: "hot_imaging")            // generation, load and battery state (figure)
//   #value("part.RW_X.mass")                       // one value with its unit, for prose
//
// Every function returns a `figure` of the right kind (table or image), so numbering and `@ref` follow your own
// document. Table captions are placed above the table. Pass `case: auto` (the default) to get every exported case.
// `revision` is the design revision the files were written from; compare it with `fune status` to find stale tables.

#let _data = json("data.json")

/// The design revision the exported files were written from.
#let revision = _data.revision

#let _at(x, key) = x.at(key, default: none)

#let _find(list, item, case: auto, subject: auto, variant: auto) = {
  let hits = list.filter(x => (
    x.item == item
      and (case == auto or _at(x, "case") == case)
      and (subject == auto or _at(x, "subject") == subject)
      and (variant == auto or _at(x, "variant") == variant)
  ))
  if hits.len() == 0 {
    panic(
      "iwafune: nothing exported for " + item + " (case: " + repr(case) + ", link: " + repr(subject)
        + "). Run `fune report export` with this item and case.",
    )
  }
  hits
}

#let _notes(notes) = if notes.len() > 0 {
  block(width: 100%, inset: (top: 4pt), align(left, text(size: 0.8em, notes.join(linebreak()))))
}

#let _unavailable(reason) = block(inset: 6pt, stroke: 0.5pt + luma(140), text(size: 0.9em, reason))

#let _align(a) = if a == "right" { right } else if a == "center" { center } else { left }

#let _head(c) = {
  let unit = _at(c, "unit")
  if unit == none { strong(c.header) } else { [#strong(c.header) \ #text(size: 0.85em)[[#unit]]] }
}

#let _cell(c, body) = if _at(c, "detail") == none { body } else {
  [#body \ #text(size: 0.8em, fill: luma(90))[#c.detail]]
}

#let _body(t) = {
  let n = t.columns.len()
  let cells = ()
  for r in t.rows {
    if r.style == "group" {
      cells.push(table.cell(colspan: n, fill: luma(238), strong(r.cells.at(0).text)))
    } else {
      if r.style in ("subtotal", "total") { cells.push(table.hline(stroke: 0.4pt)) }
      let fill = if r.style == "total" { luma(238) } else { none }
      for c in r.cells {
        let body = if r.style in ("subtotal", "total") { strong(c.text) } else if r.style == "margin" { emph(c.text) } else { c.text }
        cells.push(table.cell(fill: fill, _cell(c, body)))
      }
    }
  }
  table(
    columns: if n > 4 { (1fr,) + (auto,) * (n - 1) } else { n },
    align: t.columns.map(c => _align(c.align)),
    stroke: none,
    inset: (x: 5pt, y: 3pt),
    table.hline(stroke: 0.8pt),
    table.header(..t.columns.map(_head)),
    table.hline(stroke: 0.5pt),
    ..cells,
    table.hline(stroke: 0.8pt),
  )
}

#let _table-figure(t) = {
  set figure.caption(position: top)
  show figure: set block(breakable: true)
  figure(kind: table, caption: t.caption, {
    set text(size: 0.9em)
    if _at(t, "unavailable") != none { _unavailable(t.unavailable) } else { _body(t) }
    _notes(t.notes)
  })
}

#let _image-figure(f) = {
  set figure.caption(position: bottom)
  figure(kind: image, caption: f.caption, {
    if _at(f, "unavailable") != none { _unavailable(f.unavailable) } else { image(f.file, width: 100%) }
  })
  _notes(f.notes)
}

/// Mass budget table: items per subsystem, subtotals, total and margin rows.
#let mass-table() = for t in _find(_data.tables, "mass") { _table-figure(t) }

/// Power by mode: components by modes, with mode totals with and without margin.
#let mode-power-table() = for t in _find(_data.tables, "mode-power") { _table-figure(t) }

/// Link budget: an input table and a result table per link. `link` is `"<name>@<station>"`.
#let link-budget(link: auto) = {
  for t in _find(_data.tables, "link-budget", subject: link) { _table-figure(t) }
}

/// Node temperature table with the cases as columns, per analysis case.
#let thermal-result(case: auto) = for t in _find(_data.tables, "thermal-result", case: case) { _table-figure(t) }

/// Node temperatures over time (one figure per analysis case).
#let thermal-history(case: auto) = for f in _find(_data.figures, "thermal-history", case: case) { _image-figure(f) }

/// Generation, load and battery state of charge over time (one figure per analysis case).
#let power-history(case: auto) = for f in _find(_data.figures, "power-history", case: case) { _image-figure(f) }

#let _value(key) = {
  let hits = _data.values.filter(v => v.key == key)
  if hits.len() == 0 {
    panic("iwafune: no exported value `" + key + "`. Keys look like part.<ID>.mass, budget.mass.total, kpi.<analysis>.<kpi>.")
  }
  hits.first()
}

/// One exported value for prose, e.g. `#value("part.RW_X.mass")` gives "620 g". With `unit: false` only the number.
#let value(key, unit: true) = if unit { _value(key).text } else { _value(key).number }

/// The SI number of an exported value (for calculations in Typst).
#let value-si(key) = _value(key).si
