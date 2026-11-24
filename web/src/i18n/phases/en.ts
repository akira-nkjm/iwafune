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
    orbitReplaced: "Orbit replaced",
    orbitFrom: (maneuver: string) => `Orbit from ${maneuver}`,
    lawFor: (modes: string) => `Attitude law: ${modes}`,
    cases: "Analysis cases",
    file: "File",
    none: "None",
    editOverrides: "Edit overrides",
    overridesNote:
      "The orbit and attitude law overrides replace the design values only for the analysis cases that name this phase. Edit them with Edit overrides.",
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
    overridesDialog: {
      title: (id: string) => `Overrides of phase ${id}`,
      description:
        "Value fields are edited in place. Typed text is read by the service exactly like fune phase set-field.",
      setSummary: (id: string, path: string, text: string) => `Phase ${id}: ${path} = ${text}`,
      orbit: {
        title: "Orbit override",
        caption: "Replaces the design orbit (conditions/orbit.toml) as a whole (no per-key merge)",
        none: "No override; the design orbit is used. Copying makes the current design orbit the starting point of the override.",
        empty: "No fields are written",
        copy: "Copy the design orbit to start an override",
        recopy: "Copy the design orbit again",
        remove: "Remove override",
        copySummary: (id: string) => `Copy the design orbit into phase ${id}`,
        removeSummary: (id: string) => `Remove the orbit override of phase ${id}`,
      },
      law: {
        title: "Attitude law overrides",
        caption: "Per mode, used instead of the design attitude law",
        none: "No overrides",
        noModes: "No mode is left to override",
        mode: "Mode",
        kind: "Attitude law",
        add: "Add override",
        remove: "Remove",
        removeOf: (mode: string) => `Remove the attitude law override of mode ${mode}`,
        removeSummary: (id: string, mode: string) =>
          `Remove the attitude law override of mode ${mode} in phase ${id}`,
      },
    },
    removeDialog: {
      title: (id: string) => `Remove phase ${id}`,
      description: "The removal is refused while an analysis case still names this phase.",
      confirm: "Remove",
      summary: (id: string) => `Remove phase ${id}`,
    },
  },
};
