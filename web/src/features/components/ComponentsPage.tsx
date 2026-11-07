import { Plus, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { useParams } from "react-router";
import { useDesign } from "@/api/queries";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { DetailLayout, DetailLoading } from "@/kit/DetailLayout";
import { Empty } from "@/kit/Empty";
import { NativeSelect } from "@/kit/NativeSelect";
import { PageTabs } from "@/kit/PageTabs";
import { TextInput } from "@/kit/TextInput";
import { useDetailRoute } from "@/kit/use-detail-route";
import { groupsOf, nodeName } from "@/lib/design";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { AddPartDialog } from "./AddPartDialog";
import { PartDetail } from "./PartDetail";
import { PartsOverview } from "./PartsOverview";
import { PartsTable } from "./PartsTable";
import { PartsTree } from "./PartsTree";

type Selection = {
  view: string;
  tree: boolean;
  selected: string | null;
  onSelect: (id: string | null) => void;
};

/** 表示方法（タブ）。一覧とツリーは詳細を横の欄に、表はモーダルに出す */
const VIEWS = ["overview", "tree", "table"] as const;
type Filters = {
  search: string;
  group: string;
  setSearch: (value: string) => void;
  setGroup: (value: string) => void;
};

/** 一覧・ツリー・表のタブ。表はモーダル、一覧とツリーはサイドパネルに詳細を出す。選択と表示方法は URL に載せる。 */
export function ComponentsPage() {
  const design = useDesign();
  const route = useDetailRoute("/design/components", VIEWS);
  const { partId } = useParams();
  const [search, setSearch] = useState("");
  const [group, setGroup] = useState("");
  const filters = { search, setSearch, group, setGroup };
  const selection = {
    view: route.view,
    tree: route.tree,
    selected: partId ?? null,
    onSelect: route.select,
  };
  return (
    <>
      <ComponentsHeader
        design={design.data}
        filters={filters}
        onAdd={() => route.setAdding(true)}
      />
      <ComponentsTabs view={route.view} />
      <ComponentsContent design={design} filters={filters} selection={selection} />
      {design.data && route.adding && (
        <AddPartDialog
          design={design.data}
          open
          onOpenChange={route.setAdding}
          initialGroup={group || null}
          onCreated={(id) => {
            setSearch("");
            setGroup("");
            route.select(id);
          }}
        />
      )}
    </>
  );
}

function ComponentsHeader({
  design,
  filters,
  onAdd,
}: {
  design?: DesignView;
  filters: Filters;
  onAdd: () => void;
}) {
  const t = useT().components;
  return (
    <PageHeader
      eyebrow={t.eyebrow}
      title={t.title}
      subtitle={design ? t.subtitle(design.parts.length, groupsOf(design).length) : undefined}
      actions={
        <>
          <div className="relative">
            <Search size={13} className="absolute top-2.5 left-2.5 text-ink-4" />
            <TextInput
              aria-label={t.searchLabel}
              placeholder={t.searchPlaceholder}
              className="w-52 pl-7"
              value={filters.search}
              onChange={(e) => filters.setSearch(e.target.value)}
            />
          </div>
          <NativeSelect
            aria-label={t.groupFilter}
            value={filters.group}
            onChange={(e) => filters.setGroup(e.target.value)}
            placeholder={t.allGroups}
            options={(design ? groupsOf(design) : []).map((g) => ({
              value: g.id,
              label: nodeName(g),
            }))}
          />
          <Button onClick={onAdd} disabled={!design}>
            <Plus size={14} />
            {t.add}
          </Button>
        </>
      }
    />
  );
}

function ComponentsTabs({ view }: { view: string }) {
  const t = useT().components;
  const label = { overview: t.tabOverview, tree: t.tabTree, table: t.tabTable };
  return (
    <PageTabs
      tabs={VIEWS.map((v) => ({ to: `?view=${v}`, label: label[v], active: view === v }))}
    />
  );
}

function ComponentsContent({
  design,
  filters,
  selection,
}: {
  design: ReturnType<typeof useDesign>;
  filters: Filters;
  selection: Selection;
}) {
  const t = useT();
  const d = design.data;
  if (design.isError) return <Empty>{t.common.cannotLoadDesign(design.error.message)}</Empty>;
  if (!d) return <DetailLoading tree={selection.tree} />;
  const current = d.parts.find((p) => p.id === selection.selected);
  const overview = selection.view === "overview";
  const close = () => selection.onSelect(null);
  return (
    <DetailLayout
      tree={selection.tree}
      wideList={overview}
      title={current?.id ?? t.components.title}
      hint={t.components.selectHint}
      onClose={close}
      list={<ComponentsList design={d} filters={filters} selection={selection} />}
      detail={
        current ? (
          <PartDetail
            key={current.id}
            design={d}
            row={current}
            onSelect={selection.onSelect}
            onClose={overview ? close : undefined}
          />
        ) : null
      }
    />
  );
}

function ComponentsList({
  design,
  filters,
  selection,
}: {
  design: DesignView;
  filters: Filters;
  selection: Selection;
}) {
  const t = useT();
  const { search, group } = filters;
  const parts = useMemo(
    () => filterParts(design.parts, search, group),
    [design.parts, search, group],
  );
  if (!parts.length) return <Empty>{t.components.noMatch}</Empty>;
  if (selection.view === "overview")
    return (
      <PartsOverview
        design={design}
        parts={parts}
        selected={selection.selected}
        onSelect={selection.onSelect}
      />
    );
  if (selection.tree)
    return (
      <PartsTree
        key={JSON.stringify([search, group])}
        design={design}
        parts={parts}
        selected={selection.selected}
        onSelect={selection.onSelect}
      />
    );
  return (
    <PartsTable
      design={design}
      parts={parts}
      selected={selection.selected}
      onSelect={selection.onSelect}
    />
  );
}

function filterParts(parts: PartRow[], search: string, group: string) {
  const s = search.trim().toLowerCase();
  return parts.filter(
    (p) =>
      (!group || p.subsystem === group || p.parent === group) &&
      (!s ||
        [p.id, p.kind, p.catalog ?? "", p.file].some((value) => value.toLowerCase().includes(s))),
  );
}
