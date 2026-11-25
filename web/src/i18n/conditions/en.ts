import type { conditionsJa } from "./ja";

export const conditionsEn: typeof conditionsJa = {
  conditions: {
    eyebrow: "Conditions",
    cases: "Analysis cases",
    casesSubtitle: "cases/<ID>.toml (mode timeline, uncertainty side, orbit, environment)",
    noCases: "No analysis cases (analyses run with the default case)",
    name: "Name",
    description: "Description",
    modes: "Modes",
    file: "File",
    phase: "Phase",
    setPhase: "Choose phase",
    epoch: "Start time",
    setEpoch: "Enter start time",
    caseEpoch: {
      title: (id: string) => `Start time of case ${id}`,
      description:
        "Time-series analyses start this case's orbit at this time (same as fune case set-epoch). Leave it empty to start at the orbit's own epoch.",
      field: "Start time",
      hint: "ISO 8601 (e.g. 2031-05-02T00:00:00Z). For an orbit with a one-off span, a time inside the span.",
      confirm: "Save",
      summary: (id: string, epoch: string) =>
        epoch
          ? `Set the start time of case ${id} to ${epoch}`
          : `Clear the start time of case ${id}`,
    },
    fromGeometry: {
      open: "Cases from the worst days",
      title: "Cases from the worst days",
      description:
        "Creates three cases from the mission geometry of the phase's orbit, starting at the day of the farthest and nearest Sun distance and the farthest Earth distance (an existing case gets the new start time; same as fune case from-geometry).",
      phase: "Phase",
      prefix: "Case ID prefix (optional)",
      prefixHint:
        "<prefix>_sun_far, <prefix>_sun_near, <prefix>_earth_far. Defaults to the phase ID in lower case",
      mode: "Mode (optional)",
      noMode: "Unspecified (an existing case keeps its mode)",
      confirm: "Create cases",
      summary: (phase: string) =>
        phase
          ? `Create cases from the worst days of phase ${phase}`
          : "Create cases from the worst days",
    },
    casePhase: {
      title: (id: string) => `Phase of case ${id}`,
      description:
        "The orbit and attitude law overrides of the chosen phase are used when this case is analyzed (same as fune case set-phase).",
      field: "Phase",
      none: "No phase (the design orbit and attitude laws)",
      confirm: "Save",
      summary: (id: string, phase: string) =>
        phase ? `Set the phase of case ${id} to ${phase}` : `Clear the phase of case ${id}`,
    },
    addCase: {
      open: "Add a case",
      title: "Add an analysis case",
      description:
        "Writes cases/<ID>.toml (same as fune case set). A case with the same ID is replaced and the fields you leave out are removed (the preview lists them).",
      summary: (id: string) => `Add analysis case ${id}`,
      confirm: "Add",
      label: "Display name (optional)",
      descriptionLabel: "Description (optional)",
      mode: "Mode (optional)",
      phase: "Phase (optional)",
      side: "Uncertainty side (optional)",
      thermalCase: "Thermal environment (optional)",
      beta: "Beta angle (optional)",
      quantityHint: "Write it with a unit (e.g. 60 deg)",
      orbits: "Number of orbits (optional)",
      sunDistance: "Distance from the Sun (optional)",
      epoch: "Start time (optional)",
      epochHint: "ISO 8601. For an orbit with a one-off span, a time inside the span.",
      none: "Not specified",
    },
    links: {
      add: "Add a link",
      addTitle: "Add a comm link",
      addDescription:
        "Adds a link to analysis/comm.toml (same as fune link add). Values such as losses and the modulation are written in the settings afterwards.",
      addSummary: (id: string) => `Add comm link ${id}`,
      addConfirm: "Add",
      direction: "Direction",
      part: "Transceiver (spacecraft side)",
      noPart: "There is no transceiver part. Add one first.",
      band: "Band (ground station side)",
      bandHint: "Choose from the bands a ground station has a performance for.",
      noBand:
        "No ground station has a band performance. Write one in the station's settings first.",
      choose: "Choose one",
      editTitle: (id: string) => `Edit comm link ${id}`,
      editDescription:
        "Rewrites the direction, transceiver and band of the link (same as fune set analysis.comm.link.<name>.<field>). Changing the ID also rewrites the downlink choice and the keys of requirement checks.",
      editSummary: (id: string) => `Edit comm link ${id}`,
      editConfirm: "Save",
      panelTitle: "Comm links",
      panelCaption: (file: string) =>
        `[link.<name>] in ${file}. Values such as losses are written in the settings tab.`,
      noLinks: "There are no comm links",
      id: "ID",
      edit: (id: string) => `Edit link ${id}`,
      remove: (id: string) => `Remove link ${id}`,
      removeTitle: (id: string) => `Remove comm link ${id}`,
      removeDescription: (file: string) =>
        `Removes the link from ${file}. The downlink choice is cleared if it names this link. It cannot be removed while a requirement's check names it (the preview shows why). Undo is in the change history.`,
      removeSummary: (id: string) => `Remove comm link ${id}`,
      removeConfirm: "Remove",
    },
    openWith: "Open with this case",
    caseActions: {
      rename: (id: string) => `Rename case ${id}`,
      renameTitle: (id: string) => `Rename case ${id}`,
      renameDescription:
        "The default case (analysis.common.default_case) and the cases of requirements (quantify.cases) are rewritten too.",
      newId: "New ID",
      renameConfirm: "Rename",
      renameSummary: (id: string, to: string) => `Analysis case ${id} → ${to}`,
      remove: (id: string) => `Remove case ${id}`,
      removeTitle: (id: string) => `Remove analysis case ${id}`,
      removeDescription: (file: string) =>
        `Removes ${file}. It cannot be removed while a requirement or a setting refers to the case (the preview shows why). Undo is in the change history.`,
      removeSummary: (id: string) => `Remove analysis case ${id}`,
      removeConfirm: "Remove",
      actions: "Actions",
    },
    orbit: "Orbit & ground stations",
    orbitSubtitle:
      "conditions/ settings (orbit, ground stations, period) and ground station visibility",
    conditionsSettings: "Conditions settings",
    conditionsSettingsCaption: "Settings whose key starts with conditions.",
    noConditionsSettings: "No conditions/ settings",
    eclipseSeries: "Beta angle and eclipse seasons",
    eclipseApproximation:
      "The Sun is fixed within each sampled orbit. Eclipse is umbra plus penumbra; the penumbra series excludes umbra. Extrema are selected from the sampled dates.",
    createEclipseCase: "Create this case",
    eclipseLongest: "Longest eclipse date",
    eclipseBetaMin: "Minimum beta date",
    caseId: "New case ID",
    caseMode: "Mode (optional)",
    noCaseMode: "Unspecified",
    eclipseCaseDescription:
      "Recalculate the conditions and save the selected beta angle and date as a case.",
    visibilityPerStation: "Visibility per ground station",
    stations: {
      add: "Add ground station",
      addTitle: "Add ground station",
      addDescription:
        "Creates conditions/ground/<ID>.toml. Per-band performance (antenna gain, noise temperature, …) is written in the settings afterwards.",
      addSummary: (id: string) => `Add ground station ${id}`,
      addConfirm: "Add",
      label: "Name (optional)",
      latitude: "Latitude",
      longitude: "Longitude",
      altitude: "Altitude (optional)",
      minElevation: "Minimum elevation (optional)",
      quantityHint: "Latitude and longitude are required. Write them with a unit (e.g. 35.7 deg)",
      noValues: "This ground station has no values yet",
      rename: (id: string) => `Rename ground station ${id}`,
      renameTitle: (id: string) => `Rename ground station ${id}`,
      renameDescription:
        "The list of stations the communication analysis uses (analysis.comm.stations) is rewritten too.",
      newId: "New ID",
      renameConfirm: "Rename",
      renameSummary: (id: string, to: string) => `Ground station ${id} → ${to}`,
      remove: (id: string) => `Remove ground station ${id}`,
      removeTitle: (id: string) => `Remove ground station ${id}`,
      removeDescription: (file: string) =>
        `Deletes ${file}. You can undo it from the change history.`,
      inUse:
        "The communication analysis (analysis.comm.stations) lists this ground station, so it cannot be removed. Take it out of that setting first.",
      removeSummary: (id: string) => `Remove ground station ${id}`,
      removeConfirm: "Remove",
    },
    decaySeries: "Altitude history",
    decayApproximation:
      "Near-circular estimate integrating orbit-averaged drag (NRLMSISE-00). Presets hold solar activity constant for the whole span; dated uses values looked up by date in the space-weather table.",
    decayWeatherTable: (first: string, last: string, fetched: string) =>
      `Used the space-weather table ${first} to ${last} (fetched ${fetched}). Dates outside the table use the moderate preset.`,
    decayNoWeatherTable:
      "No space-weather table, so only the preset conditions are shown (fetch it from external data in the settings).",
  },
};
