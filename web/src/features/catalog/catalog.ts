import type { CatalogItemView, CommandText, ProjectInfo } from "@/api/types";
import { localized } from "@/i18n";

/** カタログ品の表示名（表示の言語の名前。無ければ ID） */
export function itemLabel(item: Pick<CatalogItemView, "id" | "label" | "label_en">): string {
  return localized(item.label, item.label_en) || item.id;
}

/** 書き込めるカタログの名前（fune.toml の `[[catalog]]` の順。API の `ProjectInfo.catalogs` の `editable`） */
export function writableCatalogs(project: Pick<ProjectInfo, "catalogs">): string[] {
  return project.catalogs.filter((c) => c.editable).map((c) => c.name);
}

/**
 * 選んだカタログと打った名前から作る品の ID（`<カタログ名>/<名前>`。読み方の正本は fune_model::path::split_catalog_ref）。
 * ID の規則と重複はサーバー（dry_run）が検査する。Web は ID を分解しない（`CatalogItemView.{scope, name}` を使う）
 */
export function catalogItemId(scope: string, name: string): string {
  return `${scope}/${name}`;
}

/**
 * 共有カタログを足すダイアログの欄 → `fune catalog source add` と同じ打った引数（`POST /ops/from_command`）。
 * 名前か URL が空なら送らない（null）。前後の空白・空の rev の読み方と、名前の規則・重複は service が持つ
 */
export function sharedCatalogAddCommand(
  name: string,
  git: string,
  rev: string,
): CommandText | null {
  return name.trim() && git.trim() ? { command: "catalog_source_add", name, git, rev } : null;
}
