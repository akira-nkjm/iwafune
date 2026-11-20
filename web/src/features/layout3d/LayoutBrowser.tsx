import { Eye, EyeOff, Focus, Scan } from "lucide-react";
import type { DesignView, PartRow } from "@/api/types";
import { PartsTree } from "@/features/components/list/PartsTree";
import { useT } from "@/i18n";
import { type TreeNode, treeItems } from "@/kit/tree/tree-nodes";
import { cn } from "@/lib/cn";
import { nodeLabel } from "@/lib/design";
import { isIsolated, isolate, toggleHidden, type Visibility, visibilityOf } from "./visibility";

/**
 * 配置のページの左の部品のツリー（CAD のブラウザー）。部品のページと同じ製品ツリー（`PartsTree`）に、
 * 部品とフォルダーごとの目のアイコン（フォルダーは子孫をまとめて）を添える。3D に描ける部品（`placed`）だけが切り替えの対象。
 */
export function LayoutBrowser({
  design,
  placed,
  selected,
  onSelect,
  hidden,
  onHidden,
}: {
  design: DesignView;
  /** 3D に箱がある部品（取付のある部品）の ID */
  placed: ReadonlySet<string>;
  selected: string | null;
  onSelect: (id: string) => void;
  /** 隠した部品の ID */
  hidden: ReadonlySet<string>;
  onHidden: (hidden: Set<string>) => void;
}) {
  const L = useT().systems.layout3d;
  const folderLabel = (id: string) => nodeLabel(design, id);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-line border-b px-3 py-2">
        <p className="min-w-0 flex-1 truncate font-medium text-ink-2 text-xs">{L.browser}</p>
        {hidden.size > 0 && (
          <button
            type="button"
            onClick={() => onHidden(new Set())}
            className="shrink-0 rounded-control px-1.5 py-0.5 text-accent-ink text-xs hover:bg-panel-2"
          >
            {L.showAll}
          </button>
        )}
      </div>
      <div className="min-h-0 flex-1">
        <PartsTree
          all={design.parts}
          items={design.parts}
          selected={selected}
          onSelect={onSelect}
          folderLabel={folderLabel}
          compact
          renderActions={(node) => {
            const label = node.item ? node.label : folderLabel(node.label);
            return (
              <span className="flex items-center">
                <IsolateToggle
                  node={node}
                  label={label}
                  placed={placed}
                  hidden={hidden}
                  onHidden={onHidden}
                />
                <EyeToggle
                  node={node}
                  label={label}
                  placed={placed}
                  hidden={hidden}
                  onHidden={onHidden}
                />
              </span>
            );
          }}
        />
      </div>
    </div>
  );
}

/** 行の目のアイコン。3D に描ける部品が無い行（未配置の部品だけのフォルダーなど）には出さない */
function EyeToggle({
  node,
  label: name,
  placed,
  hidden,
  onHidden,
}: {
  node: TreeNode<PartRow>;
  /** 行の見出し（部品の ID か、フォルダーの表示名） */
  label: string;
  placed: ReadonlySet<string>;
  hidden: ReadonlySet<string>;
  onHidden: (hidden: Set<string>) => void;
}) {
  const L = useT().systems.layout3d;
  const ids = treeItems(node)
    .map((p) => p.id)
    .filter((id) => placed.has(id));
  if (ids.length === 0) return <span aria-hidden="true" className="size-6" />;
  const state: Visibility = visibilityOf(hidden, ids);
  const label = state === "hidden" ? L.show(name) : L.hide(name);
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={state !== "hidden"}
      title={label}
      onClick={() => onHidden(toggleHidden(hidden, ids))}
      className={cn(
        "flex size-6 items-center justify-center rounded-control hover:bg-panel-2 hover:text-ink",
        state === "shown" ? "text-ink-3" : "text-ink-2",
        state === "mixed" && "opacity-60",
      )}
    >
      {state === "hidden" ? <EyeOff size={14} /> : <Eye size={14} />}
    </button>
  );
}

/**
 * 行の「この部品だけ表示」（フォルダーは子孫だけ）。押すとほかの描ける部品を隠し、その組だけが見えている間に押すと
 * すべて表示に戻す。3D に描ける部品が無い行には出さない
 */
function IsolateToggle({
  node,
  label: name,
  placed,
  hidden,
  onHidden,
}: {
  node: TreeNode<PartRow>;
  label: string;
  placed: ReadonlySet<string>;
  hidden: ReadonlySet<string>;
  onHidden: (hidden: Set<string>) => void;
}) {
  const L = useT().systems.layout3d;
  const ids = treeItems(node)
    .map((p) => p.id)
    .filter((id) => placed.has(id));
  if (ids.length === 0) return <span aria-hidden="true" className="size-6" />;
  const all = [...placed];
  const isolated = isIsolated(hidden, all, ids);
  const label = isolated ? L.isolated : L.isolate(name);
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={isolated}
      title={label}
      onClick={() => onHidden(isolate(hidden, all, ids))}
      className={cn(
        "flex size-6 items-center justify-center rounded-control hover:bg-panel-2 hover:text-ink",
        isolated ? "bg-accent-tint text-accent-ink" : "text-ink-3",
      )}
    >
      {/* 目のマークと同じく、状態で図も変える: ふだんは枠だけ、これだけを出している間は枠の中に点 */}
      {isolated ? <Focus size={14} /> : <Scan size={14} />}
    </button>
  );
}
