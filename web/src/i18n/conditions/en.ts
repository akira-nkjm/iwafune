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
    openWith: "Open with this case",
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
