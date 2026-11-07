import type { missionJa } from "./mission.ja";

export const missionEn: typeof missionJa = {
  mission: {
    eyebrow: "Mission",
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
      "mission/ settings (orbit, ground stations, period) and ground station visibility",
    missionSettings: "Mission settings",
    missionSettingsCaption: "Settings whose key starts with mission.",
    noMissionSettings: "No mission/ settings",
    visibilityPerStation: "Visibility per ground station",
  },
};
