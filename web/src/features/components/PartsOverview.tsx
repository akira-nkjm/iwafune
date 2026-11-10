import type { DesignView, PartRow, TreeNode } from "@/api/types";
import { useLang, useT } from "@/i18n";
import { Packed } from "@/kit/Packed";
import { RoleIcon } from "@/kit/RoleIcon";
import { cn } from "@/lib/cn";
import { kindLabel, nodeName } from "@/lib/design";
import { subsystemTone } from "@/lib/subsystems";
import { type Tone, toneHex, toneInkHex, toneText, toneTint } from "@/lib/tone";

/** カード 1 枚の幅 [px] */
const CARD_WIDTH = 216;

/** 枠の中のカードの列の数（1〜2 枚はその数、3〜6 枚は 2 列、それより多ければ 3 列） */
const columnsFor = (n: number) => (n <= 2 ? Math.max(1, n) : n <= 6 ? 2 : 3);

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
  const nodes = new Map(design.tree.map((n) => [n.id, n]));
  const kindName = (kind: string | null) => (kind ? kindLabel(design.defs, kind) : "");
  const nameOf = (n: TreeNode) => nodeName(n, lang);
  // 絞り込んだ部品か、それを子孫に持つノードだけを出す
  const shown = (id: string): boolean => {
    const n = nodes.get(id);
    return rows.has(id) || !!n?.children.some(shown);
  };
  const context = { rows, nodes, kindName, nameOf, shown, selected, onSelect };
  const roots = design.tree.filter((n) => !n.parent && shown(n.id));
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
  const tone: Tone = node ? subsystemTone(row?.subsystem ?? (assembly ? null : node.id)) : "gray";
  const heading = (
    <>
      <span
        className="size-2.5 shrink-0 rounded-full"
        style={{ background: toneHex[tone] }}
        aria-hidden
      />
      <span className="text-ink-3">
        {assembly ? td.overviewAssembly : node ? td.overviewSubsystem : ""}
      </span>
      <span className={cn("font-medium", toneText[tone])}>
        {title ?? (node ? ctx.nameOf(node) : "")}
      </span>
    </>
  );
  return (
    <section
      className={cn(
        "rounded-[14px] border border-line p-4",
        nested ? "bg-panel-2/60" : "bg-panel/70 shadow-panel",
        assembly && ctx.selected === node?.id && "ring-2 ring-accent",
      )}
    >
      {assembly && node ? (
        <button
          type="button"
          className="mb-3 flex items-center gap-2 rounded text-[13px] hover:underline"
          onClick={(e) => {
            e.stopPropagation();
            ctx.onSelect(node.id);
          }}
        >
          {heading}
        </button>
      ) : (
        <h3 className="mb-3 flex items-center gap-2 text-[13px]">{heading}</h3>
      )}
      {cards.length > 0 && (
        <div
          className="grid gap-5"
          style={{ gridTemplateColumns: `repeat(${columnsFor(cards.length)}, ${CARD_WIDTH}px)` }}
        >
          {cards.map((n) => {
            const r = ctx.rows.get(n.id);
            return r ? <PartCard key={n.id} row={r} ctx={ctx} /> : null;
          })}
        </div>
      )}
      {inner.length > 0 && (
        <div className={cn("flex flex-wrap items-start gap-4", cards.length > 0 && "mt-4")}>
          {inner.map((n) => (
            <Box key={n.id} node={n} ctx={ctx} nested />
          ))}
        </div>
      )}
    </section>
  );
}

function PartCard({ row, ctx }: { row: PartRow; ctx: Ctx }) {
  const tone = subsystemTone(row.subsystem);
  const stacked = row.count > 1;
  return (
    <div className="relative" style={{ width: CARD_WIDTH }}>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          ctx.onSelect(row.id);
        }}
        title={row.catalog ?? row.kind ?? undefined}
        className={cn(
          "flex w-full items-center gap-3 rounded-[12px] border border-line bg-panel px-3 py-2.5 text-left transition-colors hover:border-line-strong",
          ctx.selected === row.id && "ring-2 ring-accent ring-offset-1",
        )}
        style={{
          boxShadow: stacked
            ? "4px 4px 0 -1px var(--color-panel), 4px 4px 0 0 var(--color-line-strong), 8px 8px 0 -1px var(--color-panel), 8px 8px 0 0 var(--color-line)"
            : undefined,
        }}
      >
        <span
          className="flex size-10 shrink-0 items-center justify-center rounded-full"
          style={{
            background: toneTint(tone, "icon"),
            color: toneInkHex[tone],
          }}
        >
          <RoleIcon role={row.role} size={20} />
        </span>
        <span className="min-w-0">
          <span className="block truncate text-[11.5px] text-ink-3">{ctx.kindName(row.kind)}</span>
          <span className="block truncate font-medium text-[14px] text-ink">{row.id}</span>
        </span>
      </button>
      {stacked && (
        <span className="num absolute -top-2.5 -right-2.5 flex size-7 items-center justify-center rounded-full border border-line bg-panel text-[12px] text-ink-2 shadow-panel">
          {row.count}
        </span>
      )}
    </div>
  );
}
