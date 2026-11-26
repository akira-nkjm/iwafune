import type { ReactNode } from "react";
import type { AnalysisState } from "@/api/queries";
import type { DefsView, DesignView, System } from "@/api/types";
import { loadLayout3D } from "@/features/layout3d/LazyLayout3D";
import { AocsModes } from "./aocs/AocsModes";
import { AddLinkButton } from "./comm/AddLinkButton";
import { commExtras } from "./comm/commExtras";
import { PowerModes } from "./power/PowerModes";
import { PowerSeries } from "./power/PowerSeries";
import { thermalExtras } from "./thermal/thermalExtras";

/** 系のページのタブの並び（出す・出さないは `systemTabs`） */
export const SYSTEM_TAB_ORDER = [
  "results",
  "maneuvers",
  "heat",
  "couplings",
  "analyses",
  "modes",
  "series",
  "settings",
] as const;
export type SystemTab = (typeof SYSTEM_TAB_ORDER)[number];

/** 登録した系だけが出すタブ（熱画像・結合と表面・解析結果・マヌーバ）。モードのタブは API（`DefsView.mode_systems`）で決まる */
export type ExtraTab = Exclude<SystemTab, "results" | "modes" | "series" | "settings">;

/** 専用の表示に渡すもの（系のページが持っているもの） */
export interface DedicatedProps {
  design: DesignView | undefined;
  /** 選んだケース（電源のモード・時系列の問い合わせに使う） */
  kase: string | null;
}

/** タブごとの追加の中身（`SystemTabs` の `extra`） */
export type Extras = Partial<Record<SystemTab, ReactNode>>;

/** 1 つの系の登録。無い欄は共通の表示（タブ・操作・追加の表示なし、先読みなし） */
export interface SystemRegistration {
  /** 共通のタブのほかに出すタブ（`SYSTEM_TAB_ORDER` の順に並ぶ） */
  tabs?: readonly ExtraTab[];
  /** モードのタブの専用の表示（共通の表の代わり）。表（部品 × モード）の代わりなので、ページの本文がそのままスクロールする */
  modes?: (props: DedicatedProps) => ReactNode;
  /** 時系列のタブの専用の表示 */
  series?: (props: DedicatedProps) => ReactNode;
  /** ページの右上の操作（ケースの選択の前に並ぶ。その系の一覧に足すボタンなど） */
  actions?: (props: DedicatedProps) => ReactNode;
  /** `tabs` と結果のタブに足す中身（解析が出そろってから出す）。`design` が届いてから呼ぶ */
  extras?: (design: DesignView, states: readonly AnalysisState[], kase: string | null) => Extras;
  /** ページを開く前に読んでおく大きなチャンク（熱の温度で塗った 3D は three.js） */
  chunks?: readonly (() => Promise<unknown>)[];
}

/**
 * **系ごとの登録表（Web で系の名前をキーにするのはこの表だけ。系の名前で分岐するのもここだけ）。** 共通のコード（`SystemTabs`・
 * `SystemPage`・ナビの先読み）はこの表に問い合わせ、系の名前とは比べない。電源は解析を回した系別のモード表示と時系列、
 * AOCS は姿勢・機器・モード別解析のカード、通信は回線を足すボタンと回線の一覧、熱は熱画像・結合と表面・解析結果のタブと追加の表示と 3D のチャンク、
 * 推進はマヌーバのタブ。足すときは、ここに 1 行足す（xtask の `web_systems` が、ほかの所の系の名前の比較を断る）。
 * 色とアイコン（全系に必要）は `lib/systems.ts`、モードのタブを持つ系・既定で隠す系などの判断は API（`DefsView`）。
 */
export const SYSTEM_REGISTRY = {
  power: {
    modes: ({ kase }) => <PowerModes kase={kase} />,
    series: ({ kase }) => <PowerSeries kase={kase} />,
  },
  aocs: {
    modes: ({ design }) => (design ? <AocsModes design={design} /> : null),
  },
  comm: {
    actions: ({ design }) => (design ? <AddLinkButton design={design} /> : null),
    extras: commExtras,
  },
  thermal: {
    tabs: ["heat", "couplings", "analyses"],
    extras: thermalExtras,
    chunks: [loadLayout3D],
  },
  propulsion: {
    tabs: ["maneuvers"],
  },
} as const satisfies Partial<Record<System, SystemRegistration>>;

/** その系の登録（無ければ undefined） */
export function systemRegistration(system: System): SystemRegistration | undefined {
  return (SYSTEM_REGISTRY as Partial<Record<System, SystemRegistration>>)[system];
}

/** その系にその追加のタブ（登録表の `tabs`）があるか */
export function hasExtraTab(system: System, tab: SystemTab): boolean {
  return (
    (systemRegistration(system)?.tabs as readonly SystemTab[] | undefined)?.includes(tab) ?? false
  );
}

/** モードの見方を持つ系だけにタブを出す。どの系が持つかは API（`DefsView.mode_systems`）が返す。 */
export function hasModes(system: System, defs: DefsView | undefined): boolean {
  return defs?.mode_systems.includes(system) ?? false;
}

/** 系にそのタブがあるか（モードのタブは API が返す系だけ、追加のタブは登録表の `tabs` の系だけ） */
export function showsTab(system: System, tab: SystemTab, defs: DefsView | undefined): boolean {
  if (tab === "modes") return hasModes(system, defs);
  if (tab === "results" || tab === "series" || tab === "settings") return true;
  return hasExtraTab(system, tab);
}

/** 系のページに出すタブ（`SYSTEM_TAB_ORDER` の順） */
export function systemTabs(system: System, defs: DefsView | undefined): SystemTab[] {
  return SYSTEM_TAB_ORDER.filter((tab) => showsTab(system, tab, defs));
}

/** URL のタブの名前を、系にあるタブに直す（無ければ結果のタブ） */
export function resolveTab(
  system: System,
  raw: string | undefined,
  defs: DefsView | undefined,
): SystemTab {
  return systemTabs(system, defs).find((tab) => tab === raw) ?? "results";
}

/** 追加の表示を持つ系の追加の中身（無ければ空） */
export function systemExtras(
  system: System,
  design: DesignView | undefined,
  states: readonly AnalysisState[],
  kase: string | null = null,
): Extras {
  const make = systemRegistration(system)?.extras;
  return design && make ? make(design, states, kase) : {};
}

/** ページを開く前に読んでおく大きなチャンクの読み込み（無ければ空） */
export function systemChunks(system: System): readonly (() => Promise<unknown>)[] {
  return systemRegistration(system)?.chunks ?? [];
}
