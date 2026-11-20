import type { ReactNode } from "react";
import type { DesignView, System } from "@/api/types";
import { AocsModes } from "./aocs/AocsModes";
import { PowerModes } from "./power/PowerModes";
import { PowerSeries } from "./power/PowerSeries";

/** 専用の表示に渡すもの（系のページが持っているもの） */
export interface DedicatedProps {
  design: DesignView | undefined;
  /** 選んだケース（電源のモード・時系列の問い合わせに使う） */
  kase: string | null;
}

/** 系が持つ専用のタブの表示（共通の表示の代わりに出す）。無いタブは共通の表示 */
export interface DedicatedViews {
  /** モードのタブ。専用の表示は、表（部品 × モード）の代わりなので、ページの本文がそのままスクロールする */
  modes?: (props: DedicatedProps) => ReactNode;
  /** 時系列のタブ */
  series?: (props: DedicatedProps) => ReactNode;
}

/**
 * **専用の表示を持つ系の登録表（系の名前で分岐するのはここだけ）。** 共通のコード（`SystemTabs`）はこの表に問い合わせ、
 * 系の名前とは比べない。電源は解析を回した系別のモード表示と時系列、AOCS は姿勢・機器・モード別解析のカード。
 * 専用の表示を足すときは、ここに 1 行足す（xtask の `web_systems` が、ほかの所の系の名前の比較を断る）。
 */
export const DEDICATED_VIEWS = {
  power: {
    modes: ({ kase }) => <PowerModes kase={kase} />,
    series: ({ kase }) => <PowerSeries kase={kase} />,
  },
  aocs: {
    modes: ({ design }) => (design ? <AocsModes design={design} /> : null),
  },
} as const satisfies Partial<Record<System, DedicatedViews>>;

/** その系の専用の表示（無ければ undefined） */
export function dedicatedViews(system: System): DedicatedViews | undefined {
  return (DEDICATED_VIEWS as Partial<Record<System, DedicatedViews>>)[system];
}
