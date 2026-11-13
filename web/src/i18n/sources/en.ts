import type { sourcesJa } from "./ja";

export const sourcesEn: typeof sourcesJa = {
  sources: {
    title: "Sources",
    subtitle: (n: number) =>
      `sources/<ID>.toml — ${n} ${n === 1 ? "entry" : "entries"} (documents, tests and judgements behind values)`,
    empty: "No sources",
    add: "Add source",
    edit: "Edit",
    id: "ID",
    kind: "Kind",
    label: "Name",
    ref: "Reference",
    refHint: "URL, path in the repository or document number",
    open: "Open",
    revision: "Revision",
    retrieved: "Retrieved",
    sha256: "SHA-256",
    sha256Hint: "SHA-256 of the retrieved file (64 hex digits)",
    distribution: "Distribution",
    date: "Date",
    dateHint: "In the form 2026-10-15",
    by: "By",
    note: "Note",
    file: "File",
    none: "Not set",
    addTitle: "Add source",
    editTitle: (id: string) => `Edit ${id}`,
    dialogDescription:
      "Write the datasheet URL, revision, retrieval date and SHA-256 here; catalog items point to this source by its ID in datasheet.",
    summarySet: (id: string) => `Write source ${id}`,
  },
};
