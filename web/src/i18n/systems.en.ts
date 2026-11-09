import type { systemsJa } from "./systems.ja";

export const systemsEn: typeof systemsJa = {
  systems: {
    eyebrow: "Subsystem analysis",
    unknown: (s: string) => `Unknown subsystem: ${s}`,
    caseLabel: "Analysis case",
    defaultCase: "Default case",
    caseOption: (label: string, id: string) => `${label} (${id})`,
    recompute: "Recompute",
    tabs: {
      results: "Results",
      breakdown: "Breakdown",
      settings: "Settings",
      series: "Time series",
    },
    specs: {
      power: {
        description: "Load per mode, generation, orbit energy balance and battery DoD",
      },
      attitude: {
        description: "RW unloading frequency, MTQ/RCS unload time and sensor exclusion angles",
      },
      cdh: {
        description: "HK and mission data generation, required storage and margin",
      },
      comm: {
        description: "Link budget, ground station contact time, downlink volume and backlog",
      },
      thermal: {
        description: "Multi-node steady and transient analysis against operating ranges",
      },
      structure: {
        description:
          "3D layout, mass properties, quasi-static loads and simple natural frequencies",
      },
    },
    kpis: {
      power: {
        load: "Peak mode load",
        gen: "Orbit-average generation",
        balance: "Orbit-average balance (worst)",
        dod: "Max DoD",
      },
      attitude: {
        interval: "Unload interval",
        peak: "Peak momentum",
        mtq: "MTQ unload time",
        fov: "Sensor intrusion (max)",
      },
      cdh: {
        total: "Generated per day",
        storage: "Storage required",
        margin: "Storage margin",
      },
      comm: {
        margin: "Link margin (min)",
        contact: "Contact per day",
        capacity: "Downlink capacity per day",
        backlog: "Max backlog",
      },
      thermal: {
        max: "Max temperature",
        min: "Min temperature",
        tmax: "Transient max temperature",
      },
      structure: {
        mass: "Total mass",
        freq: "Lowest natural frequency",
        ms: "Minimum margin of safety",
      },
    },
    status: {
      ok: "Done",
      missing_inputs: "Missing inputs",
      failed: "Failed",
      running: "Running…",
      error: "Error",
    },
    steady: "Steady",
    timeSeries: "Time series",
    missing: (n: number) => `Missing inputs (${n})`,
    openBreakdown: "Show breakdown",
    elapsed: "Elapsed [s]",
    partsAttrs: "Parts × this subsystem's attributes",
    partsAttrsCaption:
      "Click a cell to edit it directly (SetValue). Per-state values open to edit each state",
    noParts: "No parts relate to this subsystem",
    part: "Part",
    kind: "Kind",
    settingsTitle: (system: string) => `${system} settings`,
    settingsCaption: (files: string) => `Settings in ${files}`,
    noSettings: "No settings for this subsystem yet (Project / Settings lists every setting)",
    inputOf: (title: string) => `Input passed to ${title}`,
    noSeries: "No time-series results yet (graphs appear here once a time-series analysis runs)",
    seriesSep: ", ",
    seriesUnit: (u: string) => ` [${u}]`,
    rangeSep: " – ",
    heatTitle: "Node temperatures (steady)",
    heatLegendBefore: "Border: yellow = within",
    heatLegendAfter: "of a limit, red = out of range",
    node: "Node",
    thermal3d: "3D colored by temperature",
    thermal3dLegend: "Max temperature per node (nodes whose ID matches a part ID)",
    layout3d: {
      title: "3D layout",
      parts: (n: number) => `${n} parts`,
      unplacedCount: (n: number) => ` · ${n} unplaced`,
      com: " · red dot = center of mass",
      operation: "Operation",
      translate: "Move",
      rotate: "Rotate",
      snap: "Snap",
      empty: "No part has derived body coordinates yet (set a mount and position to see it)",
      moved: (id: string) => `Moved ${id}`,
      placementOf: (id: string) => `Placement of ${id}`,
      hint: "Select a box to edit its mount, position and orientation numerically.",
      unplaced: "Unplaced",
      axes: "Axes, normals, boresights",
      axesLegend:
        "blue = spin axis, cyan = MTQ axis, yellow = normal, red = boresight, navy = thrust",
    },
  },
};
