import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { PowerModeView, SystemModesView } from "@/api/types";

/**
 * 電源のモードごとの電力の表示（`POST /system_modes/power` を解析つきで。選んだケースで）。モードタブと時系列タブが同じ問い合わせを
 * 共有する。ケースを変えている間は前の値を出したままにする（`q.systemModes` の `placeholderData`）。
 */
export function usePowerModes(kase: string | null) {
  return useQuery(q.systemModes("power", [], { kase, analyzed: true }));
}

/** 解析に基づく電力の表示を持つモード（API が設計のモードの順で返す）。 */
export function powerModesOf(view: SystemModesView): PowerModeView[] {
  return view.modes.flatMap((m) => (m.power ? [m.power] : []));
}
