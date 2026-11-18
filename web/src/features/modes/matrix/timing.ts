import type { CommandText, ModeHeader, ModeStartView, ModeWhen } from "@/api/types";

/** 時刻の指定の欄の値（打った文字列。数として読むのは service。`fune mode timing` と同じ） */
export interface TimingFields {
  /** `any` は「指定なし」と同じなので、送るときは省く */
  when: ModeWhen | null;
  start: ModeStartView["from"] | null;
  offset: string;
}

/** モードの今の時刻の指定 → 欄の値 */
export function timingFields(mode: Pick<ModeHeader, "when" | "start">): TimingFields {
  return {
    when: mode.when ?? null,
    start: mode.start?.from ?? null,
    offset: mode.start?.offset?.display ?? "",
  };
}

/**
 * 欄の値 → `mode_timing` の引数。`any` と空は旗を書かないのと同じ。オフセットは開始の基準があるときだけ送る
 * （`fune mode timing <モード> --when … --start … --offset …`）。
 */
export function timingCommand(mode: string, fields: TimingFields): CommandText {
  const offset = fields.offset.trim();
  return {
    command: "mode_timing",
    mode,
    when: fields.when && fields.when !== "any" ? fields.when : null,
    start: fields.start,
    offset: fields.start && offset ? offset : null,
  };
}

/** 指定が何も無いか（`any` だけも無いのと同じ） */
export function isUnspecified(fields: TimingFields): boolean {
  return (fields.when === null || fields.when === "any") && fields.start === null;
}
