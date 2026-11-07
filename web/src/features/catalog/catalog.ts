import type { CatalogItemView } from "@/api/types";
import { localized } from "@/i18n";

/** カタログ品の表示名（表示の言語の名前。無ければ ID） */
export function itemLabel(item: Pick<CatalogItemView, "id" | "label" | "label_en">): string {
  return localized(item.label, item.label_en) || item.id;
}
