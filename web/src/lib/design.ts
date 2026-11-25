import type {
  AttributeDefView,
  DefsView,
  DesignView,
  KindDefView,
  PartRow,
  System,
  TreeNode,
  ValueView,
} from "@/api/types";
import { dictionaries, getLang, type Lang, localized } from "@/i18n";

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

/**
 * 「主（補足）」の表示: 選択肢の「ID（表示名）」、「名前（種類）」など。**ID と名前を並べるのはここだけ**
 * （括弧の書き方は i18n の `common.withNote`。日本語は全角、英語は半角）。補足が無いか主と同じなら主だけ。
 */
export function withNote(
  main: string,
  note: string | null | undefined,
  lang: Lang = getLang(),
): string {
  return note?.trim() && note !== main ? dictionaries[lang].common.withNote(main, note) : main;
}

/** 日英の表示名を持つ定義（属性・種別） */
type Labeled = { id: string; label: string; label_en: string | null };

/** 定義の表示名（言語で選ぶ。英語名が無ければ日本語、どちらも無ければ ID） */
export function defName(def: Labeled): string {
  return localized(def.label, def.label_en) || def.id;
}

/** 選択肢に出す「ID（表示名）」（`withNote`）。表示名が ID と同じなら ID だけ */
export function defWithId(def: Labeled): string {
  return withNote(def.id, defName(def));
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

/** 解析の系の表示名（`labels.section`。言語に合わせる。届くまでと表に無いときは系の綴り） */
export function systemTitle(system: System, defs?: Pick<DefsView, "labels">): string {
  const label = defs?.labels.section[system];
  return label ? localized(label.label, label.label_en) : system;
}

/** 系と組立品・入れ子のフォルダの選択肢。未作成の系も API の一覧から選べる。 */
export function parentOptions(design: DesignView): { value: string; label: string }[] {
  return [
    ...design.defs.systems.map((system) => ({
      value: system,
      label: systemTitle(system, design.defs),
    })),
    ...design.tree
      .filter((n) => n.parent !== null || n.node === "part")
      .map((n) => ({ value: n.id, label: idWithName(n) })),
  ];
}

/** 表示用の系別ツリー。parts/ 直下の部品も API の系にまとめる（置き場所は変更しない）。 */
export function systemTree(design: DesignView): TreeNode[] {
  const nodes = new Map(design.tree.map((n) => [n.id, { ...n, children: [...n.children] }]));
  for (const part of design.parts) {
    if (part.parent || !part.system) continue;
    let group = nodes.get(part.system);
    if (!group) {
      const label = design.defs.labels.section[part.system];
      group = {
        id: part.system,
        node: "group",
        system: part.system,
        label: label?.label ?? part.system,
        label_en: label?.label_en ?? null,
        parent: null,
        children: [],
      };
      nodes.set(group.id, group);
    }
    const node = nodes.get(part.id);
    if (node && group.node === "group") {
      node.parent = group.id;
      if (!group.children.includes(part.id)) group.children.push(part.id);
    }
  }
  return [...nodes.values()];
}

/** ID と、設計に書いた表示名（日本語）を持つもの: 製品ツリーのノード・モードなど */
export type Named = { id: string; label?: string | null; label_en?: string | null };

/**
 * 製品ツリーのノードやモードの表示名。系は API の日英名を使う。
 * 英語名を持たない部品やモードは英語で ID、日本語で設計の表示名（無い・空・空白だけなら ID）を使う。
 * 空の表示名を ID に替える規則は Rust の `fune_model::display_name` と同じ（`display_name_cases.json` で揃える）。
 */
export function nodeName(node: Named, lang: Lang = getLang()): string {
  const name = lang === "en" ? node.label_en : node.label;
  return name?.trim() ? name : node.id;
}

/** 選択肢に出す「ID（表示名）」（`withNote`）。表示名が ID と同じとき（英語・表示名なし）は ID だけ */
export function idWithName(node: Named, lang: Lang = getLang()): string {
  return withNote(node.id, nodeName(node, lang), lang);
}

/** ID からノードの表示名（`nodeName`）。ID が無ければ「—」、ツリーに無ければ ID */
export function nodeLabel(design: DesignView, id: string | null, lang: Lang = getLang()): string {
  if (!id) return "—";
  const system = design.defs.systems.find((s) => s === id);
  if (system) return systemTitle(system, design.defs);
  const node = design.tree.find((n) => n.id === id);
  return node ? nodeName(node, lang) : id;
}

/**
 * 形態 `configuration` が部品の配置を上書きしているか。取付のある部品は API の `mount.overridden`、
 * 取付の無い部品（根）には取付が無いので、形態の一覧（`overridden`）から読む。
 */
export function poseOverridden(
  design: Pick<DesignView, "configurations">,
  configuration: string | null | undefined,
  part: Pick<PartRow, "id" | "mount">,
): boolean {
  if (!configuration) return false;
  if (part.mount) return part.mount.overridden;
  return Boolean(
    design.configurations.find((c) => c.id === configuration)?.overridden.includes(part.id),
  );
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
