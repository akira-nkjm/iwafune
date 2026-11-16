import type { CatalogItemView, PartRow, System } from "@/api/types";

/** 表示専用の階層。item が無い節はフォルダー。 */
export type TreeNode<T> = {
  key: string;
  label: string;
  item?: T;
  children: TreeNode<T>[];
};

function folder<T>(roots: TreeNode<T>[], path: string[]): TreeNode<T>[] {
  let children = roots;
  let key = "folder";
  for (const label of path) {
    key += `/${label}`;
    let node = children.find((n) => n.key === key);
    if (!node) {
      node = { key, label, children: [] };
      children.push(node);
    }
    children = node.children;
  }
  return children;
}

/**
 * ファイルの階層（API の `dir`。区間は製品ツリーのノードの ID）を作り、組立品の子は親部品の下に置く。循環は階層にしない。
 * `parts/` の直下の部品は系（`system`）の節に置く。パスの文字列は分解しない。
 */
export function partsTree<T extends Pick<PartRow, "id" | "parent" | "dir" | "system">>(
  parts: readonly T[],
): TreeNode<T>[] {
  const roots: TreeNode<T>[] = [];
  const nodes = new Map(
    parts.map((p) => [
      p.id,
      { key: `item/${p.id}`, label: p.id, item: p, children: [] } as TreeNode<T>,
    ]),
  );
  for (const part of parts) {
    const node = nodes.get(part.id)!;
    const seen = new Set([part.id]);
    let parent = part.parent;
    while (parent && nodes.has(parent) && !seen.has(parent)) {
      seen.add(parent);
      parent = nodes.get(parent)?.item?.parent ?? null;
    }
    if (part.parent && nodes.has(part.parent) && !(parent && seen.has(parent))) {
      nodes.get(part.parent)!.children.push(node);
    } else {
      const path = part.dir.length ? part.dir : part.system ? [part.system] : [];
      folder(roots, path).push(node);
    }
  }
  return roots;
}

/**
 * カタログのツリーの分け方（`catalog` はカタログ名、`kind` は種別定義の親子、`tag` はタグごと（複数のタグを持つ品は各タグの下）、
 * `system` は API の `CatalogItemView.system`、ほかは値ごと）
 */
export const CATALOG_GROUPINGS = [
  "catalog",
  "manufacturer",
  "kind",
  "maturity",
  "tag",
  "system",
] as const;
export type CatalogGrouping = (typeof CATALOG_GROUPINGS)[number];

/** フォルダーの見出しを作る側（Web は表示名を持たない。呼び手が API の定義から渡す） */
export type CatalogTreeLabels = {
  /** 値が無い品を入れるフォルダーの見出し */
  unset: string;
  /** 種別の表示名 */
  kind: (id: string) => string;
  /** 成熟度の表示名 */
  maturity: (value: string) => string;
  /** 系の表示名（API の `DefsView` の系の名前） */
  system: (id: System) => string;
  /** タグの無い品を入れるフォルダーの見出し */
  noTags: string;
  /** 系の無い品（役割の無い種別）を入れるフォルダーの見出し */
  unassigned: string;
};

/** 分け方の 1 段。`id` が null の段は「未設定」。フォルダーの key は段の id の並びから作る（文字列は分解しない） */
type Step = { id: string | null; label: string };

type CatalogTreeItem = Pick<
  CatalogItemView,
  "id" | "scope" | "name" | "kind" | "manufacturer" | "maturity" | "tags" | "system"
>;

/** `kind` の祖先から自身までの種別 ID（根から順。定義に無い種別は自身だけ。循環は打ち切る） */
function kindChain(kinds: ReadonlyMap<string, string | null>, kind: string): string[] {
  const chain = [kind];
  let parent = kinds.get(kind) ?? null;
  while (parent && !chain.includes(parent)) {
    chain.unshift(parent);
    parent = kinds.get(parent) ?? null;
  }
  return chain;
}

/** 品が入るフォルダーの経路（段の並び）の一覧。多くは 1 つで、`tag` は持つタグの数だけ（タグ無しは 1 つ） */
function groupPaths(
  item: CatalogTreeItem,
  by: CatalogGrouping,
  kinds: ReadonlyMap<string, string | null>,
  labels: CatalogTreeLabels,
): Step[][] {
  if (by === "tag") {
    const tags = [...new Set(item.tags)];
    return tags.length
      ? tags.map((tag) => [{ id: tag, label: tag }])
      : [[{ id: null, label: labels.noTags }]];
  }
  return [groupPath(item, by, kinds, labels)];
}

function groupPath(
  item: CatalogTreeItem,
  by: CatalogGrouping,
  kinds: ReadonlyMap<string, string | null>,
  labels: CatalogTreeLabels,
): Step[] {
  const unset: Step = { id: null, label: labels.unset };
  const plain = (value: string | null | undefined, label = value): Step =>
    value ? { id: value, label: label ?? value } : unset;
  switch (by) {
    case "catalog":
      return [plain(item.scope)];
    case "manufacturer":
      return [plain(item.manufacturer), plain(item.kind, labels.kind(item.kind))];
    case "kind":
      return item.kind
        ? kindChain(kinds, item.kind).map((k) => ({ id: k, label: labels.kind(k) }))
        : [unset];
    case "maturity":
      return [plain(item.maturity, item.maturity && labels.maturity(item.maturity))];
    case "system":
      return [
        item.system
          ? { id: item.system, label: labels.system(item.system) }
          : { id: null, label: labels.unassigned },
      ];
    case "tag":
      return [];
  }
}

/**
 * カタログ品を `by` の分け方でフォルダーにまとめる。値の無い品は「未設定」のフォルダー（根では末尾）。
 * `kind` は種別定義の親子（`kinds` の `parent`）を入れ子のフォルダーにする。フォルダーの key は段の値の並びの JSON で一意。
 */
export function catalogTree<T extends CatalogTreeItem>(
  items: readonly T[],
  by: CatalogGrouping,
  kinds: readonly { id: string; parent: string | null }[],
  labels: CatalogTreeLabels,
): TreeNode<T>[] {
  const roots: TreeNode<T>[] = [];
  const parents = new Map(kinds.map((k) => [k.id, k.parent]));
  const unsetKeys = new Set<string>();
  for (const item of items) {
    const paths = groupPaths(item, by, parents, labels);
    for (const path of paths) {
      let children = roots;
      const ids: (string | null)[] = [];
      for (const step of path) {
        ids.push(step.id);
        const key = `folder/${JSON.stringify(ids)}`;
        let node = children.find((n) => n.key === key);
        if (!node) {
          node = { key, label: step.label, children: [] };
          children.push(node);
          if (step.id === null) unsetKeys.add(key);
        }
        children = node.children;
      }
      // 複数のフォルダーに入る品は、フォルダーごとに key を分ける
      const key = paths.length > 1 ? `item/${JSON.stringify(ids)}/${item.id}` : `item/${item.id}`;
      children.push({ key, label: item.name || item.id, item, children: [] });
    }
  }
  const rank = (node: TreeNode<T>) => (unsetKeys.has(node.key) ? 1 : 0);
  return roots.sort((a, b) => rank(a) - rank(b));
}

/** 一致する項目と祖先を残し、結果までの経路を自動展開する。 */
export function filterTree<T>(nodes: readonly TreeNode<T>[], matches: (item: T) => boolean) {
  const expanded = new Set<string>();
  function visit(branch: readonly TreeNode<T>[]): TreeNode<T>[] {
    return branch.flatMap((node) => {
      const children = visit(node.children);
      if (!(node.item && matches(node.item)) && children.length === 0) return [];
      if (children.length) expanded.add(node.key);
      return [{ ...node, children }];
    });
  }
  return { nodes: visit(nodes), expanded };
}

/** フォルダーの件数は子孫の項目数。 */
export function treeCount<T>(node: TreeNode<T>): number {
  return (node.item ? 1 : 0) + node.children.reduce((sum, child) => sum + treeCount(child), 0);
}

/** 節とその子孫の項目（フォルダーの表示・非表示などをまとめて切り替えるため）。 */
export function treeItems<T>(node: TreeNode<T>): T[] {
  return [...(node.item ? [node.item] : []), ...node.children.flatMap(treeItems)];
}

/** 一致する項目までの祖先の節の key（根から順に。項目自身は含めない。無ければ空）。 */
export function ancestorKeys<T>(
  nodes: readonly TreeNode<T>[],
  matches: (item: T) => boolean,
): string[] {
  for (const node of nodes) {
    if (node.item && matches(node.item)) return [];
    const below = ancestorKeys(node.children, matches);
    if (below.length || node.children.some((c) => c.item && matches(c.item)))
      return [node.key, ...below];
  }
  return [];
}
