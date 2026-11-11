import { ChevronDown, ChevronRight, Folder } from "lucide-react";
import { type HTMLAttributes, type KeyboardEvent, type ReactNode, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { type TreeNode, treeCount } from "./tree-nodes";

type Row<T> = { node: TreeNode<T>; level: number; parent?: string; position: number; size: number };
type TreeProps<T> = {
  nodes: readonly TreeNode<T>[];
  expanded?: ReadonlySet<string>;
  selected: string | null;
  onSelect: (id: string) => void;
  label: string;
  renderItem: (item: T, label: string) => ReactNode;
  folderLabel?: (name: string) => string;
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
  const toggle = (key: string) =>
    setCollapsed((old) => {
      const next = new Set(old);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
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
  return { rows, open, attributes };
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
    </div>
  );
}

/** 共通の単一選択ツリー。フォーカスと選択を分け、矢印・Home/End・Enter に対応。 */
export function Tree<T extends { id: string }>(props: TreeProps<T>) {
  const tree = useTree(props);
  return (
    <div role="tree" aria-label={props.label} className="h-full overflow-auto py-1">
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
  );
}
