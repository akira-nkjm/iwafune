import type { DesignView, PartRow, TreeNode } from "@/api/types";
import { useLang, useT } from "@/i18n";
import { CARD_WIDTH, CardGrid, NestedBoxes } from "@/kit/cards/CardGrid";
import { GroupBox } from "@/kit/cards/GroupBox";
import { ItemCard } from "@/kit/cards/ItemCard";
import { Packed } from "@/kit/Packed";
import { kindLabel, nodeName, systemTree } from "@/lib/design";
import { systemTone } from "@/lib/systems";
import type { Tone } from "@/lib/tone";

/**
 * コンポーネントの「一覧」のタブ: サブシステム（Group）の枠ごとに部品のカードを並べる（製品ツリーの形のまま）。
 * カードは役割のアイコン・種類（種別定義の表示名）・ID と、員数（2 以上のとき右上の数。重ねて描く）。組立品は枠の中の
 * 小さな枠で、見出しを押すと組立品そのものを選び、子の部品をその中に並べる。サブシステムの枠は隙間が少なくなるよう
 * 詰めて並べる（`kit/Packed.tsx`）。表示するのは絞り込んだ部品（`items`）と、それを含む枠だけ。
 * 枠の名前は表示の言語の名前だけ（英語は ID、日本語は表示名）。
 */
export function PartsOverview({
  design,
  items,
  selected,
  onSelect,
}: {
  design: DesignView;
  /** 絞り込んだ部品（出すカード） */
  items: readonly PartRow[];
  selected: string | null;
  onSelect: (id: string | null) => void;
}) {
  const lang = useLang();
  const td = useT().components;
  const rows = new Map(items.map((p) => [p.id, p]));
  const tree = systemTree(design);
  const nodes = new Map(tree.map((n) => [n.id, n]));
  const kindName = (kind: string | null) => (kind ? kindLabel(design.defs, kind) : "");
  const nameOf = (n: TreeNode) => nodeName(n, lang);
  // 絞り込んだ部品か、それを子孫に持つノードだけを出す
  const shown = (id: string): boolean => {
    const n = nodes.get(id);
    return rows.has(id) || !!n?.children.some(shown);
  };
  const context = { rows, nodes, kindName, nameOf, shown, selected, onSelect };
  const roots = tree.filter((n) => !n.parent && shown(n.id));
  const loose = roots.filter((n) => n.node === "part");
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: 空いた所を押すと選択を外す（カードはボタンで選べる）
    // biome-ignore lint/a11y/useKeyWithClickEvents: 同上
    <div className="h-full overflow-auto p-6" onClick={() => onSelect(null)}>
      <Packed gap={20}>
        {roots
          .filter((n) => n.node === "group")
          .map((g) => (
            <Box key={g.id} node={g} ctx={context} />
          ))}
        {loose.length > 0 && (
          <Box
            key="(loose)"
            node={null}
            title={td.overviewUnassigned}
            looseParts={loose}
            ctx={context}
          />
        )}
      </Packed>
    </div>
  );
}

interface Ctx {
  rows: Map<string, PartRow>;
  nodes: Map<string, TreeNode>;
  /** 種別の表示名（種別の無い部品は空） */
  kindName: (kind: string | null) => string;
  nameOf: (n: TreeNode) => string;
  shown: (id: string) => boolean;
  selected: string | null;
  onSelect: (id: string | null) => void;
}

/** サブシステムの枠か組立品の小さな枠（`node` が null なら未所属の部品の枠） */
function Box({
  node,
  title,
  looseParts,
  ctx,
  nested = false,
}: {
  node: TreeNode | null;
  title?: string;
  looseParts?: TreeNode[];
  ctx: Ctx;
  nested?: boolean;
}) {
  const td = useT().components;
  const children = (looseParts ?? node?.children.map((id) => ctx.nodes.get(id)) ?? []).filter(
    (n): n is TreeNode => !!n && ctx.shown(n.id),
  );
  // 子を持たない部品はカード、子を持つ部品（組立品）と入れ子の Group は小さな枠
  const cards = children.filter((n) => n.node === "part" && n.children.length === 0);
  const inner = children.filter((n) => n.node === "group" || n.children.length > 0);
  const assembly = node?.node === "part";
  const row = node ? ctx.rows.get(node.id) : undefined;
  const tone: Tone = node ? systemTone(row?.system ?? node.system) : "gray";
  const prefix = assembly ? td.overviewAssembly : node ? td.overviewSubsystem : "";
  return (
    <GroupBox
      tone={tone}
      prefix={prefix}
      title={title ?? (node ? ctx.nameOf(node) : "")}
      nested={nested}
      selected={assembly && ctx.selected === node?.id}
      onSelectHeading={assembly && node ? () => ctx.onSelect(node.id) : undefined}
    >
      {cards.length > 0 && (
        <CardGrid count={cards.length}>
          {cards.map((n) => {
            const r = ctx.rows.get(n.id);
            return r ? <PartCard key={n.id} row={r} ctx={ctx} /> : null;
          })}
        </CardGrid>
      )}
      {inner.length > 0 && (
        <NestedBoxes afterCards={cards.length > 0}>
          {inner.map((n) => (
            <Box key={n.id} node={n} ctx={ctx} nested />
          ))}
        </NestedBoxes>
      )}
    </GroupBox>
  );
}

function PartCard({ row, ctx }: { row: PartRow; ctx: Ctx }) {
  return (
    <ItemCard
      role={row.role}
      tone={systemTone(row.system)}
      caption={ctx.kindName(row.kind)}
      name={row.id}
      count={row.count}
      title={row.catalog ?? row.kind ?? undefined}
      selected={ctx.selected === row.id}
      onSelect={() => ctx.onSelect(row.id)}
      width={CARD_WIDTH}
    />
  );
}
