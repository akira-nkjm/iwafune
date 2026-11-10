import type { projectJa } from "./project.ja";

export const projectEn: typeof projectJa = {
  project: {
    eyebrow: "Project",
    diagnostics: "Diagnostics",
    diagSubtitle: (e: number, w: number, i: number) => `${e} errors · ${w} warnings · ${i} info`,
    filterBySeverity: "Filter by severity",
    cannotLoadDiagnostics: (msg: string) => `Cannot load diagnostics: ${msg}`,
    noDiagnostics: "No matching diagnostics",
    settings: "Settings",
    data: {
      title: "External data",
      ephemeris: { de440: "Ephemeris: DE440", analytic: "Ephemeris: analytic" },
      installed: "Installed",
      cached: "Cached",
      missing: "Missing",
      fetching: "Fetching…",
      fetch: "Fetch",
    },
    projectPanel: "Project",
    name: "Name",
    root: "Location",
    branch: "Branch",
    notGit: "(not a git repository)",
    revision: "Revision",
    schema: "Schema version",
    groups: {
      mission: { title: "Mission", caption: "mission/ — orbit, ground stations, period" },
      analysis: { title: "Analysis", caption: "analysis/ — settings of each subsystem analysis" },
      budget: { title: "Budget", caption: "budget/ — allocations and margin policy" },
    },
    noSettings: "No settings",
    key: "Key",
    value: "Value",
    file: "File",
  },
};
