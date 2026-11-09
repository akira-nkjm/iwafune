import type { System } from "@/api/types";
import { getT, localized } from "@/i18n";
import { EACH, type FieldPath } from "./output";

/** 系のページの KPI: どの解析の、どの欄を、どう集めるか */
export interface KpiSpec {
  /** 表示名は i18n の systems.kpis.<系>.<key>（kpiLabel） */
  key: string;
  analysis: string;
  /** 出力の欄の指し方（output.ts の pickNumbers。`EACH` は配列の全要素）。複数並べると、すべてから集めて pick する */
  paths: readonly FieldPath[];
  pick: "max" | "min" | "first";
  /** fields.ts の欄の表示名 */
  field: string;
}

export interface SystemSpec {
  /** 名前は defs.labels.section、説明は i18n の systems.specs.<key>。設定は API の `SettingView.system` で選ぶ */
  key: System;
  kpis: KpiSpec[];
}

export const systems: Record<System, SystemSpec> = {
  power: {
    key: "power",
    kpis: [
      {
        key: "load",
        analysis: "power.mode_load",
        paths: [["modes", EACH, "total_with_growth"]],
        pick: "first",
        field: "total_with_growth",
      },
      {
        key: "gen",
        analysis: "power.orbit_generation",
        paths: [["orbit_average"]],
        pick: "first",
        field: "orbit_average",
      },
      {
        key: "balance",
        analysis: "power.energy_balance",
        paths: [["cases", EACH, "orbit_average_balance"]],
        pick: "min",
        field: "orbit_average_balance",
      },
      {
        key: "dod",
        analysis: "power.energy_balance",
        paths: [["cases", EACH, "max_dod"]],
        pick: "max",
        field: "max_dod",
      },
    ],
  },
  attitude: {
    key: "attitude",
    kpis: [
      {
        key: "interval",
        analysis: "attitude.rw_unloading",
        paths: [["unload_interval"]],
        pick: "first",
        field: "unload_interval",
      },
      {
        key: "peak",
        analysis: "attitude.rw_unloading",
        paths: [["peak_momentum"]],
        pick: "first",
        field: "peak_momentum",
      },
      {
        key: "mtq",
        analysis: "attitude.unload_time",
        paths: [["mtq", "time"]],
        pick: "first",
        field: "time",
      },
      {
        key: "fov",
        analysis: "attitude.sensor_fov",
        paths: [["sensors", EACH, "intrusion_fraction"]],
        pick: "max",
        field: "intrusion_fraction",
      },
    ],
  },
  cdh: {
    key: "cdh",
    kpis: [
      {
        key: "total",
        analysis: "cdh.data_budget",
        paths: [["total_per_day"]],
        pick: "first",
        field: "total_per_day",
      },
      {
        key: "storage",
        analysis: "cdh.data_budget",
        paths: [["storage_required"]],
        pick: "first",
        field: "storage_required",
      },
      {
        key: "margin",
        analysis: "cdh.data_budget",
        paths: [["storage_margin"]],
        pick: "first",
        field: "storage_margin",
      },
    ],
  },
  comm: {
    key: "comm",
    kpis: [
      {
        key: "margin",
        analysis: "comm.link_budget",
        paths: [["links", EACH, "margin_db"]],
        pick: "min",
        field: "margin_db",
      },
      {
        key: "contact",
        analysis: "comm.visibility",
        paths: [["combined", "contact_per_day"]],
        pick: "first",
        field: "contact_per_day",
      },
      {
        key: "capacity",
        analysis: "comm.data_throughput",
        paths: [["capacity_per_day"]],
        pick: "first",
        field: "capacity_per_day",
      },
      {
        key: "backlog",
        analysis: "comm.data_throughput",
        paths: [["max_backlog"]],
        pick: "first",
        field: "max_backlog",
      },
    ],
  },
  thermal: {
    key: "thermal",
    kpis: [
      {
        key: "max",
        analysis: "thermal.steady",
        paths: [["envelope", EACH, "max"]],
        pick: "max",
        field: "temperature",
      },
      {
        key: "min",
        analysis: "thermal.steady",
        paths: [["envelope", EACH, "min"]],
        pick: "min",
        field: "temperature",
      },
      {
        key: "tmax",
        analysis: "thermal.transient",
        paths: [["nodes", EACH, "max"]],
        pick: "max",
        field: "temperature",
      },
    ],
  },
  structure: {
    key: "structure",
    kpis: [
      {
        key: "mass",
        analysis: "structure.mass_properties",
        paths: [["summary", "total_mass"]],
        pick: "first",
        field: "total_mass",
      },
      {
        key: "freq",
        analysis: "structure.modal_estimate",
        paths: [["results", EACH, "frequency"]],
        pick: "min",
        field: "frequency",
      },
      {
        key: "ms",
        analysis: "structure.static_load",
        paths: [
          ["mounts", EACH, "margin_axial"],
          ["mounts", EACH, "margin_lateral"],
          ["mounts", EACH, "margin_moment"],
        ],
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
