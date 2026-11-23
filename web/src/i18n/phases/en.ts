import type { phasesJa } from "./ja";

/** Phases page (English) */
export const phasesEn: typeof phasesJa = {
  phases: {
    title: "Phases",
    subtitle:
      "phases/<ID>.toml (mission stages: order, start and end conditions, duration, modes used, orbit and attitude law overrides)",
    empty: "No phases",
    add: "Add phase",
    edit: "Edit",
    remove: "Remove",
    position: "#",
    id: "ID",
    name: "Name",
    order: "Order",
    start: "Starts when",
    end: "Ends when",
    duration: "Duration",
    modes: "Modes used",
    overrides: "Overrides",
    orbit: "Orbit",
    law: "Attitude law",
    cases: "Analysis cases",
    file: "File",
    none: "None",
    overridesNote:
      "The orbit and attitude law overrides are shown only. Edit their values with fune phase set-field (saving the heading fields here keeps them).",
    dialog: {
      addTitle: "Add phase",
      editTitle: (id: string) => `Edit phase ${id}`,
      description:
        'Typed quantities ("30 d") and integers are read by the service exactly like fune phase add|set. The orbit and attitude law overrides are kept as they are.',
      label: "Name",
      order: "Order (an integer; blank is 0)",
      start: "Starts when",
      end: "Ends when",
      duration: "Duration",
      modes: "Modes used (comma-separated)",
      confirmAdd: "Add",
      confirmEdit: "Save",
      summaryAdd: (id: string) => `Add phase ${id}`,
      summaryEdit: (id: string) => `Rewrite phase ${id}`,
    },
    removeDialog: {
      title: (id: string) => `Remove phase ${id}`,
      description: "The removal is refused while an analysis case still names this phase.",
      confirm: "Remove",
      summary: (id: string) => `Remove phase ${id}`,
    },
  },
};
