import { api } from "@/api/client";
import type { FormatItem, FormatRequest, FormatResponse } from "@/api/types";
import { getT } from "@/i18n";

/**
 * 量の表示文字列を POST /units/format にまとめて問い合わせる（sat_units::format が単位と桁を選ぶ）。
 * 同じ描画の間に集まった値を 1 回の要求にまとめる（1 ページに数十の値があっても要求は 1 つ）。
 */
type Waiter = { item: FormatItem; resolve: (text: string) => void; reject: (e: unknown) => void };

let queue: Waiter[] = [];
let timer: ReturnType<typeof setTimeout> | null = null;

/** まとめる待ち時間 [ms] */
const WINDOW = 8;

async function flush(): Promise<void> {
  const batch = queue;
  queue = [];
  timer = null;
  if (batch.length === 0) return;
  try {
    const body: FormatRequest = { items: batch.map((w) => w.item) };
    const res = await api<FormatResponse>("/units/format", { method: "POST", body });
    batch.forEach((w, i) => {
      const r = res.items[i];
      if (r) w.resolve(r.display);
      else w.reject(new Error(getT().quantity.noText));
    });
  } catch (e) {
    for (const w of batch) w.reject(e);
  }
}

export function formatQuantity(item: FormatItem): Promise<string> {
  return new Promise((resolve, reject) => {
    queue.push({ item, resolve, reject });
    if (!timer) timer = setTimeout(() => void flush(), WINDOW);
  });
}

/** 表示文字列の問い合わせのキーの先頭（これで始まる問い合わせが、数量の表示文字列） */
export const FORMAT_KEY = "fmt";

/** キャッシュのキー（同じ値・次元・単位は 1 回だけ問い合わせる） */
export function formatKey(item: FormatItem): readonly [string, string, string | null, number] {
  return [FORMAT_KEY, item.dimension, item.unit, item.si] as const;
}
