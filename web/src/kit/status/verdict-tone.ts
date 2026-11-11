import type { Verdict } from "@/api/types";

/** 判定 → 色の役（Pill・TickBar・KPI で共通の 1 つの対応） */
export const verdictTone: Record<Verdict, "pos" | "warn" | "neg"> = {
  ok: "pos",
  warning: "warn",
  violation: "neg",
};
