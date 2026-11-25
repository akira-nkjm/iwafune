import type { Choice } from "@/api/types";
import { localized } from "@/i18n";

/** 選べるもの 1 つの表示（表示名が値と違えば `表示名 (値)`） */
export function choiceLabel(c: Choice): string {
  const name = localized(c.label, c.label_en);
  return name === c.value ? c.value : `${name} (${c.value})`;
}
