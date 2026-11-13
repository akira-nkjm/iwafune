import type { Distribution, EvidenceSource, Op, SourceKind } from "@/api/types";

/** 文字列で書く出典の欄（ファイルのキーと同じ名前。`fune source set` の引数と同じ） */
export const TEXT_FIELDS = [
  "label",
  "ref",
  "revision",
  "retrieved",
  "sha256",
  "date",
  "by",
  "note",
] as const;
export type TextField = (typeof TEXT_FIELDS)[number];

/** 編集中の出典（打ったままの文字列。空は「書かない」） */
export type SourceDraft = {
  id: string;
  kind: SourceKind;
  distribution: Distribution | "";
} & Record<TextField, string>;

/** 出典 → 編集の下書き（無い欄は空） */
export function draftOf(source: EvidenceSource | null): SourceDraft {
  const text = (f: TextField) => source?.[f] ?? "";
  return {
    id: source?.id ?? "",
    kind: source?.kind ?? "datasheet",
    distribution: source?.distribution ?? "",
    ...(Object.fromEntries(TEXT_FIELDS.map((f) => [f, text(f)])) as Record<TextField, string>),
  };
}

/**
 * 下書き → `set_source` の Op（ID が空なら null）。打った文字列は前後の空白を除いてそのまま渡し、
 * 形（日付・SHA-256）の検査はサーバーの dry_run に任せる
 */
export function setSourceOp(draft: SourceDraft): Op | null {
  const id = draft.id.trim();
  if (!id) return null;
  const value = (f: TextField) => draft[f].trim() || null;
  const source: EvidenceSource = {
    id,
    kind: draft.kind,
    distribution: draft.distribution || null,
    ...(Object.fromEntries(TEXT_FIELDS.map((f) => [f, value(f)])) as Record<
      TextField,
      string | null
    >),
  };
  return { op: "set_source", source };
}
