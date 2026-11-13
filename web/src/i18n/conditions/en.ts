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
  },
};
