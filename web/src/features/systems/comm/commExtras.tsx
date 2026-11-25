import type { DesignView } from "@/api/types";
import type { Extras } from "../dedicated";
import { LinksPanel } from "./LinksPanel";

/** 通信のページの追加の表示（結果のタブの上に回線の一覧）。登録表（`dedicated.tsx` の `SYSTEM_REGISTRY.comm.extras`）から呼ばれる */
export function commExtras(design: DesignView): Extras {
  return { results: <LinksPanel design={design} /> };
}
