import { Formatted } from "./Formatted";

/** 整形が届くまでの代わり（生の SI の数は桁が長く、単位も違って見えるので出さない） */
export const PENDING = "…";

/** 温度（SI の K）を API の整形で出す（熱画像・温度範囲の図・板の温度の分布が同じ出し方を使う） */
export function Temp({ si }: { si: number }) {
  return <Formatted si={si} dimension="Temperature" fallback={PENDING} />;
}
