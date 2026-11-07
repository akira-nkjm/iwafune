import type { System } from "@/api/types";
import { getT, localized } from "@/i18n";

/** 系のページの KPI: どの解析の、どの欄を、どう集めるか */
export interface KpiSpec {
  /** 表示名は i18n の systems.kpis.<系>.<key>（kpiLabel） */
  key: string;
  analysis: string;
  /** output.ts の pickNumbers のパス */
  /** 複数のパスを並べると、すべてから集めて pick する */
  path: string | string[];
  pick: "max" | "min" | "first";
  /** fields.ts の欄の表示名 */
  field: string;
}

export interface SystemSpec {
  /** 名前は defs.labels.section、説明は i18n の systems.specs.<key> */
  key: System;
  /** この系の設定のキーの接頭辞 */
  settings: string[];
  kpis: KpiSpec[];
}

export const systems: Record<System, SystemSpec> = {
  power: {
    key: "power",
    settings: ["analysis.power", "budget.power", "power."],
    kpis: [
      {
        key: "load",
        analysis: "power.mode_load",
        path: "modes[].total_with_growth",
        pick: "first",
        field: "total_with_growth",
      },
      {
        key: "gen",
        analysis: "power.orbit_generation",
        path: "orbit_average",
        pick: "first",
        field: "orbit_average",
      },
      {
        key: "balance",
        analysis: "power.energy_balance",
        path: "cases[].orbit_average_balance",
        pick: "min",
        field: "orbit_average_balance",
      },
      {
        key: "dod",
        analysis: "power.energy_balance",
        path: "cases[].max_dod",
        pick: "max",
        field: "max_dod",
      },
    ],
  },
  attitude: {
    key: "attitude",
    settings: ["analysis.attitude", "attitude."],
    kpis: [
      {
        key: "interval",
        analysis: "attitude.rw_unloading",
        path: "unload_interval",
        pick: "first",
        field: "unload_interval",
      },
      {
        key: "peak",
        analysis: "attitude.rw_unloading",
        path: "peak_momentum",
        pick: "first",
        field: "peak_momentum",
      },
      {
        key: "mtq",
        analysis: "attitude.unload_time",
        path: "mtq.time",
        pick: "first",
        field: "time",
      },
      {
        key: "fov",
        analysis: "attitude.sensor_fov",
        path: "sensors[].intrusion_fraction",
        pick: "max",
        field: "intrusion_fraction",
      },
    ],
  },
  cdh: {
    key: "cdh",
    settings: ["analysis.cdh", "budget.data", "cdh."],
    kpis: [
      {
        key: "total",
        analysis: "cdh.data_budget",
        path: "total_per_day",
        pick: "first",
        field: "total_per_day",
      },
      {
        key: "storage",
        analysis: "cdh.data_budget",
        path: "storage_required",
        pick: "first",
        field: "storage_required",
      },
      {
        key: "margin",
        analysis: "cdh.data_budget",
        path: "storage_margin",
        pick: "first",
        field: "storage_margin",
      },
    ],
  },
  comm: {
    key: "comm",
    settings: ["analysis.comm", "comm."],
    kpis: [
      {
        key: "margin",
        analysis: "comm.link_budget",
        path: "links[].margin_db",
        pick: "min",
        field: "margin_db",
      },
      {
        key: "contact",
        analysis: "comm.visibility",
        path: "combined.contact_per_day",
        pick: "first",
        field: "contact_per_day",
      },
      {
        key: "capacity",
        analysis: "comm.data_throughput",
        path: "capacity_per_day",
        pick: "first",
        field: "capacity_per_day",
      },
      {
        key: "backlog",
        analysis: "comm.data_throughput",
        path: "max_backlog",
        pick: "first",
        field: "max_backlog",
      },
    ],
  },
  thermal: {
    key: "thermal",
    settings: ["analysis.thermal", "thermal."],
    kpis: [
      {
        key: "max",
        analysis: "thermal.steady",
        path: "envelope[].max",
        pick: "max",
        field: "temperature",
      },
      {
        key: "min",
        analysis: "thermal.steady",
        path: "envelope[].min",
        pick: "min",
        field: "temperature",
      },
      {
        key: "tmax",
        analysis: "thermal.transient",
        path: "nodes[].max",
        pick: "max",
        field: "temperature",
      },
    ],
  },
  structure: {
    key: "structure",
    settings: ["analysis.structure", "budget.mass", "structure."],
    kpis: [
      {
        key: "mass",
        analysis: "structure.mass_properties",
        path: "summary.total_mass",
        pick: "first",
        field: "total_mass",
      },
      {
        key: "freq",
        analysis: "structure.modal_estimate",
        path: "results[].frequency",
        pick: "min",
        field: "frequency",
      },
      {
        key: "ms",
        analysis: "structure.static_load",
        path: ["mounts[].margin_axial", "mounts[].margin_lateral", "mounts[].margin_moment"],
        pick: "min",
        field: "margin_axial",
      },
    ],
  },
};

export const systemOrder: System[] = ["power", "attitude", "cdh", "comm", "thermal", "structure"];

export function isSystem(s: string | undefined): s is System {
  return !!s && (systemOrder as string[]).includes(s);
}

/** 系の名前（言語に合わせる） */
export function systemTitle(key: System, defs?: import("@/api/types").DefsView): string {
  const label = defs?.labels.section[key];
  return label ? localized(label.label, label.label_en) : key;
}

/** 系の説明（言語に合わせる） */
export function systemDescription(key: System): string {
  return getT().systems.specs[key].description;
}

/** KPI の名前（言語に合わせる） */
export function kpiLabel(system: System, kpi: string): string {
  return getT().systems.kpis[system][kpi] ?? kpi;
}
