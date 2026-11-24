import { ChevronsDownUp } from "lucide-react";
import { useState } from "react";
import type { DesignView, PartRow, TreeNode } from "@/api/types";
import { useLang, useT } from "@/i18n";
import { CARD_WIDTH, CardGrid, NestedBoxes } from "@/kit/cards/CardGrid";
import { GroupBox } from "@/kit/cards/GroupBox";
import { ItemCard } from "@/kit/cards/ItemCard";
import { DetailAction } from "@/kit/detail/DetailPane";
import { Packed } from "@/kit/Packed";
import { kindLabel, nodeName, systemTree } from "@/lib/design";
import { systemTone } from "@/lib/systems";
import type { Tone } from "@/lib/tone";
import { GroupActions } from "../groups/GroupActions";
import { foldable, foldedByDefault, partsWithin } from "./overview-fold";

/**
 * コンポーネントの「一覧」のタブ: サブシステム（Group）の枠ごとに部品のカードを並べる（製品ツリーの形のまま）。
 * カードは役割のアイコン・種類（種別定義の表示名）・ID と、員数（2 以上のとき右上の数。重ねて描く）。組立品は枠の中の
 * 小さな枠で、見出しを押すと組立品そのものを選び、子の部品をその中に並べる。サブシステムの枠は隙間が少なくなるよう
 * 詰めて並べる（`kit/Packed.tsx`）。表示するのは絞り込んだ部品（`items`）と、それを含む枠だけ。
 * 枠の名前は表示の言語の名前だけ（英語は ID、日本語は表示名）。
 * 入れ子のグループは、1 枚のカード（名前と中の部品の数）にたためる（枠の見出しを押すとたたみ、カードを押すと開く）。
 * 中が部品だけのグループは、ふだんはたたんで並べる。中に組立品や別のグループを持つグループは、ふだんは開いておく。
 * 選んでいる部品を（組立品の中も含めて）持つグループは開いておく。
 * 組立品の枠も、見出しの操作でカード 1 枚（組立品そのもののカード。右上は中の部品の数）にたためる。たたんだカードを押すと、
 * 組立品を選んで枠を開く。
 * 開いた枠には名前の変更と解除の操作が付く。
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
  onFold,
}: {
  node: TreeNode | null;
  title?: string;
  looseParts?: TreeNode[];
  ctx: Ctx;
  nested?: boolean;
  /** 渡すと、見出しを押してこの枠をカードにたためる（入れ子のグループ） */
  onFold?: () => void;
}) {
  const td = useT().components;
  // 既定の状態から押して切り替えた入れ子のグループ（この枠の中のもの。既定は、中が部品だけならたたむ、それ以外は開く）
  const [flipped, setFlipped] = useState<ReadonlySet<string>>(new Set());
  const toggle = (id: string) =>
    setFlipped((old) => {
      const next = new Set(old);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  const children = (looseParts ?? node?.children.map((id) => ctx.nodes.get(id)) ?? []).filter(
    (n): n is TreeNode => !!n && ctx.shown(n.id),
  );
  // 子を持たない部品はカード、子を持つ部品（組立品）と入れ子の Group は小さな枠
  const cards = children.filter((n) => n.node === "part" && n.children.length === 0);
  const nestedAll = children.filter((n) => n.node === "group" || n.children.length > 0);
  // たたんでカードにするグループ: たためるもののうち、いまたたむ状態で、選んでいる部品を中に持たないもの
  const folded = nestedAll.filter(
    (n) =>
      foldable(n, ctx) &&
      foldedByDefault(n, ctx) !== flipped.has(n.id) &&
      !(ctx.selected !== null && partsWithin(n, ctx).includes(ctx.selected)),
  );
  const inner = nestedAll.filter((n) => !folded.includes(n));
  const assembly = node?.node === "part";
  const row = node ? ctx.rows.get(node.id) : undefined;
  const tone: Tone = node ? systemTone(row?.system ?? node.system) : "gray";
  // 親を持つ Group は系の下の入れ子のグループ（名前の変更と解除ができる）
  const nestedGroup = node?.node === "group" && node.parent !== null;
  const prefix = assembly
    ? td.overviewAssembly
    : nestedGroup
      ? td.overviewGroup
      : node
        ? td.overviewSubsystem
        : "";
  return (
    <GroupBox
      tone={tone}
      prefix={prefix}
      title={title ?? (node ? ctx.nameOf(node) : "")}
      nested={nested}
      selected={assembly && ctx.selected === node?.id}
      onSelectHeading={
        assembly && node ? () => ctx.onSelect(node.id) : onFold ? () => onFold() : undefined
      }
      actions={
        nestedGroup && node ? (
          <GroupActions group={node} nodes={ctx.nodes} onFold={onFold} />
        ) : assembly && onFold ? (
          <FoldAction onFold={onFold} />
        ) : undefined
      }
    >
      {cards.length + folded.length > 0 && (
        <CardGrid count={cards.length + folded.length}>
          {cards.map((n) => {
            const r = ctx.rows.get(n.id);
            return r ? <PartCard key={n.id} row={r} ctx={ctx} /> : null;
          })}
          {folded.map((n) =>
            n.node === "part" ? (
              <FoldedAssemblyCard key={n.id} node={n} ctx={ctx} onOpen={() => toggle(n.id)} />
            ) : (
              <FoldedGroupCard key={n.id} node={n} ctx={ctx} onOpen={() => toggle(n.id)} />
            ),
          )}
        </CardGrid>
      )}
      {inner.length > 0 && (
        <NestedBoxes afterCards={cards.length + folded.length > 0}>
          {inner.map((n) => (
            <Box
              key={n.id}
              node={n}
              ctx={ctx}
              nested
              onFold={foldable(n, ctx) ? () => toggle(n.id) : undefined}
            />
          ))}
        </NestedBoxes>
      )}
    </GroupBox>
  );
}

/**
 * たたんだグループのカード（部品のカードと同じ大きさ）。アイコンと種類は中の部品がそろっているときだけ出し、
 * 右上の数は中の部品の数（組立品とその子も数える）。押すと枠に開く。
 */
function FoldedGroupCard({ node, ctx, onOpen }: { node: TreeNode; ctx: Ctx; onOpen: () => void }) {
  const td = useT().components;
  const rows = partsWithin(node, ctx).flatMap((id) => {
    const r = ctx.rows.get(id);
    return r ? [r] : [];
  });
  const first = rows[0];
  const same = (pick: (r: PartRow) => string | null) =>
    first !== undefined && rows.every((r) => pick(r) === pick(first));
  const kind = same((r) => r.kind) && first ? ctx.kindName(first.kind) : "";
  return (
    <ItemCard
      role={same((r) => r.role) && first ? first.role : null}
      tone={systemTone(first?.system ?? node.system)}
      caption={kind ? `${td.overviewGroup} · ${kind}` : td.overviewGroup}
      name={ctx.nameOf(node)}
      count={rows.length}
      title={rows.map((r) => r.id).join(", ")}
      selected={false}
      onSelect={onOpen}
      width={CARD_WIDTH}
    />
  );
}

/** 組立品の枠の見出しの操作: たたむ（1 枚のカードに戻す）。押しても、見出しの「組立品を選ぶ」や枠の外の選択解除は起きない */
function FoldAction({ onFold }: { onFold: () => void }) {
  const tg = useT().components.group;
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: 見出しと枠の外へ押下を伝えないための囲み
    // biome-ignore lint/a11y/useKeyWithClickEvents: 同上（中のボタンがキーで操作できる）
    <span className="ml-auto flex gap-1" onClick={(e) => e.stopPropagation()}>
      <DetailAction icon={ChevronsDownUp} label={tg.foldAction} onClick={onFold} />
    </span>
  );
}

/**
 * たたんだ組立品のカード: 組立品そのものの部品のカードで、右上の数は中の部品の数。押すと組立品を選び、枠に開く。
 */
function FoldedAssemblyCard({
  node,
  ctx,
  onOpen,
}: {
  node: TreeNode;
  ctx: Ctx;
  onOpen: () => void;
}) {
  const td = useT().components;
  const row = ctx.rows.get(node.id);
  const inside = partsWithin(node, ctx);
  const kind = row ? ctx.kindName(row.kind) : "";
  return (
    <ItemCard
      role={row?.role ?? null}
      tone={systemTone(row?.system ?? node.system)}
      caption={kind ? `${td.overviewAssembly} · ${kind}` : td.overviewAssembly}
      name={node.id}
      count={inside.length}
      title={inside.join(", ")}
      selected={ctx.selected === node.id}
      onSelect={() => {
        ctx.onSelect(node.id);
        onOpen();
      }}
      width={CARD_WIDTH}
    />
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
