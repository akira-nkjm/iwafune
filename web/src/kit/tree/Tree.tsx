import { ChevronDown, ChevronRight, ChevronsDownUp, ChevronsUpDown, Folder } from "lucide-react";
import {
  type HTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { Button } from "@/ui/button";
import { ancestorKeys, branchKeys, type TreeNode, treeCount } from "./tree-nodes";

type Row<T> = { node: TreeNode<T>; level: number; parent?: string; position: number; size: number };
type TreeProps<T> = {
  nodes: readonly TreeNode<T>[];
  expanded?: ReadonlySet<string>;
  selected: string | null;
  onSelect: (id: string) => void;
  label: string;
  renderItem: (item: T, label: string) => ReactNode;
  folderLabel?: (name: string) => string;
  /** 行の右端の操作（表示・非表示の切り替えなど。フォルダーにも出せる）。押しても行の選択・開閉は起こさない */
  renderActions?: (node: TreeNode<T>) => ReactNode;
};

function visibleRows<T>(
  nodes: readonly TreeNode<T>[],
  open: (key: string) => boolean,
  level = 1,
  parent?: string,
): Row<T>[] {
  return nodes.flatMap((node, index) => [
    { node, level, parent, position: index + 1, size: nodes.length },
    ...(open(node.key) ? visibleRows(node.children, open, level + 1, node.key) : []),
  ]);
}

function treeKeyDown<T extends { id: string }>(
  event: KeyboardEvent,
  row: Row<T>,
  index: number,
  rows: Row<T>[],
  open: (key: string) => boolean,
  toggle: (key: string) => void,
  focus: (key?: string) => void,
  select: (id: string) => void,
) {
  const { node } = row;
  switch (event.key) {
    case "ArrowDown":
      focus(rows[index + 1]?.node.key);
      break;
    case "ArrowUp":
      focus(rows[index - 1]?.node.key);
      break;
    case "Home":
      focus(rows[0]?.node.key);
      break;
    case "End":
      focus(rows.at(-1)?.node.key);
      break;
    case "ArrowRight":
      if (node.children.length && !open(node.key)) toggle(node.key);
      else if (node.children.length) focus(rows[index + 1]?.node.key);
      break;
    case "ArrowLeft":
      if (node.children.length && open(node.key)) toggle(node.key);
      else focus(row.parent);
      break;
    case "Enter":
    case " ":
      if (node.item) select(node.item.id);
      else toggle(node.key);
      break;
    default:
      return;
  }
  event.preventDefault();
}

function useTree<T extends { id: string }>(props: TreeProps<T>) {
  const [collapsed, setCollapsed] = useState(new Set<string>());
  const [focused, setFocused] = useState<string | null>(null);
  const refs = useRef(new Map<string, HTMLDivElement>());
  const open = (key: string) =>
    !collapsed.has(key) && (props.expanded ? props.expanded.has(key) : true);
  const rows = visibleRows(props.nodes, open);
  const active = rows.some((r) => r.node.key === focused)
    ? focused
    : (rows.find((r) => r.node.item?.id === props.selected) ?? rows[0])?.node.key;
  // 外で選ばれた項目が閉じたフォルダーの中なら祖先を開き、見える所まで送る
  const { nodes, selected } = props;
  useEffect(() => {
    if (selected === null) return;
    const path = ancestorKeys(nodes, (item) => item.id === selected);
    setCollapsed((old) =>
      path.some((key) => old.has(key)) ? new Set([...old].filter((k) => !path.includes(k))) : old,
    );
  }, [nodes, selected]);
  const selectedKey = rows.find((r) => r.node.item && r.node.item.id === selected)?.node.key;
  useEffect(() => {
    if (selectedKey) refs.current.get(selectedKey)?.scrollIntoView?.({ block: "nearest" });
  }, [selectedKey]);
  const toggle = (key: string) =>
    setCollapsed((old) => {
      const next = new Set(old);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  // すべて開く／閉じる。絞り込み中（expanded あり）も同じ collapsed で動く:
  // 開く = collapsed を空にして一致した枝をすべて出す、閉じる = 一致した枝も含めてすべて閉じる（最上位の行は残る）
  const branches = branchKeys(props.nodes);
  const expandAll = () => setCollapsed(new Set());
  const collapseAll = () => setCollapsed(new Set(branches));
  const focus = (key?: string) => {
    if (key) refs.current.get(key)?.focus();
  };
  function attributes(row: Row<T>, index: number) {
    return {
      ref: (element: HTMLDivElement | null) => {
        if (element) refs.current.set(row.node.key, element);
        else refs.current.delete(row.node.key);
      },
      tabIndex: active === row.node.key ? 0 : -1,
      onFocus: () => setFocused(row.node.key),
      onKeyDown: (event: KeyboardEvent) =>
        treeKeyDown(event, row, index, rows, open, toggle, focus, props.onSelect),
      onClick: (event: React.MouseEvent) => {
        focus(row.node.key);
        const chevron =
          row.node.children.length > 0 &&
          event.target instanceof Element &&
          event.target.closest("[data-chevron]");
        if (row.node.item && !chevron) props.onSelect(row.node.item.id);
        else if (row.node.children.length) toggle(row.node.key);
      },
    };
  }
  return {
    rows,
    open,
    attributes,
    branchCount: branches.length,
    allOpen: collapsed.size === 0,
    allClosed: branches.every((key) => collapsed.has(key)),
    expandAll,
    collapseAll,
  };
}

function TreeChevron({ open, expandable }: { open: boolean; expandable: boolean }) {
  return (
    <span
      data-chevron
      aria-hidden="true"
      className="flex size-3 shrink-0 items-center justify-center"
    >
      {expandable ? open ? <ChevronDown size={14} /> : <ChevronRight size={14} /> : null}
    </span>
  );
}

function TreeRow<T extends { id: string }>({
  row,
  open,
  attributes,
  props,
}: {
  row: Row<T>;
  open: boolean;
  attributes: HTMLAttributes<HTMLDivElement> & { ref: (element: HTMLDivElement | null) => void };
  props: TreeProps<T>;
}) {
  const { node } = row;
  const label = node.item ? node.label : (props.folderLabel?.(node.label) ?? node.label);
  return (
    <div
      {...attributes}
      role="treeitem"
      tabIndex={attributes.tabIndex}
      aria-label={label}
      aria-level={row.level}
      aria-posinset={row.position}
      aria-setsize={row.size}
      aria-expanded={node.children.length ? open : undefined}
      aria-selected={node.item ? props.selected === node.item.id : undefined}
      style={{ paddingLeft: row.level * 10 }}
      className={cn(
        "flex min-h-9 cursor-pointer items-center gap-1.5 border-line-soft border-b pr-3 text-xs outline-offset-[-2px] hover:bg-panel-2 focus-visible:outline-2 focus-visible:outline-accent",
        props.selected === node.item?.id && "bg-accent-tint",
      )}
    >
      <TreeChevron open={open} expandable={node.children.length > 0} />
      {node.item ? (
        props.renderItem(node.item, node.label)
      ) : (
        <>
          <Folder size={14} className="shrink-0 text-ink-3" />
          <span className="truncate" title={label}>
            {label}
          </span>
          <span className="shrink-0 text-ink-3 tabular-nums">{treeCount(node)}</span>
        </>
      )}
      {props.renderActions && (
        // 行の操作は行の選択・開閉・矢印の移動に渡さない
        // biome-ignore lint/a11y/noStaticElementInteractions: 中のボタンの押下とキーを行に伝えないための囲み
        <span
          className="ml-auto flex shrink-0 items-center"
          onClick={(event) => event.stopPropagation()}
          onKeyDown={(event) => event.stopPropagation()}
        >
          {props.renderActions(node)}
        </span>
      )}
    </div>
  );
}

/** 「すべて開く」「すべて閉じる」。 */
export function TreeToolbar({
  allOpen,
  allClosed,
  onExpandAll,
  onCollapseAll,
}: {
  allOpen: boolean;
  allClosed: boolean;
  onExpandAll: () => void;
  onCollapseAll: () => void;
}) {
  const t = useT().common;
  return (
    <div className="flex shrink-0 items-center justify-end gap-0.5 border-line-soft border-b px-2 py-0.5">
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={t.expandAll}
        title={t.expandAll}
        disabled={allOpen}
        onClick={onExpandAll}
      >
        <ChevronsUpDown />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={t.collapseAll}
        title={t.collapseAll}
        disabled={allClosed}
        onClick={onCollapseAll}
      >
        <ChevronsDownUp />
      </Button>
    </div>
  );
}

/**
 * 共通の単一選択ツリー。フォーカスと選択を分け、矢印・Home/End・Enter に対応。
 * 外から選ばれた項目（3D で選んだ部品など）は、祖先のフォルダーを開いて見える所まで送る。
 */
export function Tree<T extends { id: string }>(props: TreeProps<T>) {
  const tree = useTree(props);
  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* フォルダー（子を持つ節）が無いツリーには出さない */}
      {tree.branchCount > 0 && (
        <TreeToolbar
          allOpen={tree.allOpen}
          allClosed={tree.allClosed}
          onExpandAll={tree.expandAll}
          onCollapseAll={tree.collapseAll}
        />
      )}
      <div role="tree" aria-label={props.label} className="min-h-0 flex-1 overflow-auto py-1">
        {tree.rows.map((row, index) => (
          <TreeRow
            key={row.node.key}
            row={row}
            open={tree.open(row.node.key)}
            attributes={tree.attributes(row, index)}
            props={props}
          />
        ))}
      </div>
    </div>
  );
}
