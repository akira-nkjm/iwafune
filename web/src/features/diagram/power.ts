import type { DesignView, NetView } from "@/api/types";
import { getT } from "@/i18n";
import { toneHex, toneInkHex, uiHex } from "@/lib/tone";

/** 系統図の層（表示の切り替え） */
export type Layer = "power" | "signal" | "gnd";
/** 図の層（ツールバーの並び順） */
export const LAYERS: readonly Layer[] = ["power", "signal", "gnd"];

export interface PowerCategory {
  key: string;
  label: string;
  color: string;
  /** 並べ順（凡例） */
  order: number;
}

/** 系統の定義（名前は呼んだときの言語で引く） */
function fixedCategories() {
  const c = getT().diagram.categories;
  return {
    gnd: { key: "gnd", label: c.gnd, color: uiHex.ink3, order: 90 },
    battery: { key: "battery", label: c.battery, color: toneHex.navy, order: 10 },
    solar: { key: "solar", label: c.solar, color: toneHex.blue, order: 20 },
    unregulated: { key: "unregulated", label: c.unregulated, color: toneHex.red, order: 30 },
  } satisfies Record<string, PowerCategory>;
}
/** 安定化レールの色（電圧の高い順に割り当てる。パレットの色だけを使う） */
const RAIL_COLORS = [
  toneHex.gold,
  toneHex.teal,
  toneHex.rose,
  toneInkHex.blue,
  toneInkHex.gold,
  toneInkHex.teal,
];

/** ネットの層: 戻り（API の `is_return`。判定は fune_model の Net::is_return）は GND、ほかの電源は給電、データと RF は信号 */
export function layerOf(n: NetView): Layer {
  if (n.kind !== "power") return "signal";
  return n.is_return ? "gnd" : "power";
}

/**
 * 電源ネットの系統（凡例と線の色）。表示だけの分類で、設計データには書かない。
 * どの系統かは API の `NetView.power_class`（役割・戻り・レールから service が決める）。ここは色・凡例の並び・名前を
 * 当てるだけで、安定化レールは電圧ごと（高い順）に色を分ける。
 */
export function powerCategories(design: DesignView): Map<string, PowerCategory> {
  const out = new Map<string, PowerCategory>();
  const { gnd: GND, battery: BATTERY, solar: SOLAR, unregulated: UNREG } = fixedCategories();
  const rails: { net: NetView; si: number; display: string }[] = [];
  for (const n of design.nets) {
    if (n.kind !== "power") continue;
    switch (n.power_class) {
      case "return":
        out.set(n.id, GND);
        break;
      case "battery":
        out.set(n.id, BATTERY);
        break;
      case "solar":
        out.set(n.id, SOLAR);
        break;
      case "regulated":
        if (n.rail) rails.push({ net: n, si: n.rail.si, display: n.rail.display });
        else out.set(n.id, UNREG);
        break;
      default:
        out.set(n.id, UNREG);
    }
  }
  const volts = [...new Set(rails.map((r) => r.display))].sort(
    (a, b) =>
      (rails.find((r) => r.display === b)?.si ?? 0) - (rails.find((r) => r.display === a)?.si ?? 0),
  );
  for (const r of rails) {
    const i = volts.indexOf(r.display);
    out.set(r.net.id, {
      key: `rail:${r.display}`,
      label: getT().diagram.categories.regulated(r.display),
      color: RAIL_COLORS[i % RAIL_COLORS.length] ?? uiHex.ink3,
      order: 40 + i,
    });
  }
  return out;
}
