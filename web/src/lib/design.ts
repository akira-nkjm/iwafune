import type {
  AttributeDefView,
  DefsView,
  DesignView,
  KindDefView,
  PartRow,
  TreeNode,
  ValueView,
} from "@/api/types";
import { getLang, type Lang, localized } from "@/i18n";

/**
 * 設計データ（GET /design）からの引き当て。表示の組み立てだけで、値の計算はしない。
 */
export function attrOf(part: PartRow, id: string): ValueView | undefined {
  return part.values[id];
}

/**
 * パラメーターのパス（`RW_X.mass`、`RW_X.power.on`）。API の `paths` に無ければ null。記法の正本は fune_model::path で、
 * Web はつながない（同じ編集で定義を作る属性は、その定義を当てた部品 `POST /parts/{id}` の `paths` から引く）
 */
export function findParamPath(
  owner: Pick<PartRow, "paths">,
  attr: string,
  state?: string | null,
): string | null {
  return owner.paths[attr]?.[state ?? ""] ?? null;
}

/** パラメーターのパス（`findParamPath`。API の `paths` に無ければ属性 ID を見出しに出す） */
export function paramPath(
  part: Pick<PartRow, "paths">,
  attr: string,
  state?: string | null,
): string {
  return findParamPath(part, attr, state) ?? attr;
}

/** 部品のフィールドのパス（`RW_X.count`）。API の `field_paths`（名前は fune_model::field_names::PartFieldName）。無ければ null */
export function fieldPath(part: Pick<PartRow, "field_paths">, field: string): string | null {
  return part.field_paths[field] ?? null;
}

/** 日英の表示名を持つ定義（属性・種別） */
type Labeled = { id: string; label: string; label_en: string | null };

/** 定義の表示名（言語で選ぶ。英語名が無ければ日本語、どちらも無ければ ID） */
export function defName(def: Labeled): string {
  return localized(def.label, def.label_en) || def.id;
}

/** 選択肢に出す「ID（表示名）」。表示名が ID と同じなら ID だけ */
export function defWithId(def: Labeled): string {
  const name = defName(def);
  return name !== def.id ? `${def.id}（${name}）` : def.id;
}

/** 属性定義（GET /design の defs）。無ければ undefined */
export function attrDef(
  defs: Pick<DefsView, "attributes">,
  id: string,
): AttributeDefView | undefined {
  return defs.attributes.find((a) => a.id === id);
}

/** 属性の表示名（定義が無ければ ID） */
export function attrLabel(defs: Pick<DefsView, "attributes">, id: string): string {
  const def = attrDef(defs, id);
  return def ? defName(def) : id;
}

/** 種別定義。無ければ undefined */
export function kindDef(defs: Pick<DefsView, "kinds">, id: string): KindDefView | undefined {
  return defs.kinds.find((k) => k.id === id);
}

/** 種別の表示名（定義が無ければ ID） */
export function kindLabel(defs: Pick<DefsView, "kinds">, id: string): string {
  const def = kindDef(defs, id);
  return def ? defName(def) : id;
}

/** 製品ツリーの Group（サブシステムなど） */
export function groupsOf(design: DesignView): TreeNode[] {
  return design.tree.filter((n) => n.node === "group");
}

/** ID と、設計に書いた表示名（日本語）を持つもの: 製品ツリーのノード・モードなど */
export type Named = { id: string; label?: string | null };

/**
 * 製品ツリーのノード（Group・部品）やモードの表示名。表示の言語の名前だけにする: 英語は ID（AOCS など。設計に書く
 * 表示名は日本語のため）、日本語は設計に書いた表示名（無ければ ID）。
 */
export function nodeName(node: Named, lang: Lang = getLang()): string {
  return lang === "en" || !node.label ? node.id : node.label;
}

/** 選択肢に出す「ID（表示名）」。表示名が ID と同じとき（英語・表示名なし）は ID だけ */
export function idWithName(node: Named, lang: Lang = getLang()): string {
  const name = nodeName(node, lang);
  return name !== node.id ? `${node.id}（${name}）` : node.id;
}

/** ID からノードの表示名（`nodeName`）。ID が無ければ「—」、ツリーに無ければ ID */
export function nodeLabel(design: DesignView, id: string | null, lang: Lang = getLang()): string {
  if (!id) return "—";
  const node = design.tree.find((n) => n.id === id);
  return node ? nodeName(node, lang) : id;
}

export function partById(design: DesignView): Map<string, PartRow> {
  return new Map(design.parts.map((p) => [p.id, p]));
}

/** ポートの種類はネットの種類から決める（電源ネットの供給元なら出力） */
export type PortRole = "power_in" | "power_out" | "data" | "rf" | "none";

export function portRole(
  design: DesignView,
  part: string,
  port: string,
  net: string | null,
): PortRole {
  if (!net) return "none";
  const n = design.nets.find((x) => x.id === net);
  if (!n) return "none";
  if (n.kind === "power")
    return n.source?.part === part && n.source.port === port ? "power_out" : "power_in";
  if (n.kind === "rf") return "rf";
  return "data";
}
