import { Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useDesign } from "@/api/queries";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { SearchInput } from "@/kit/forms/SearchInput";
import { matchesSearch } from "@/kit/forms/search";
import { ListDetailPage } from "@/kit/list-detail/ListDetailPage";
import { type DetailRoute, useDetailRoute } from "@/kit/list-detail/use-detail-route";
import { PageTabs } from "@/kit/selection/PageTabs";
import { nodeLabel, systemTitle } from "@/lib/design";
import { paths, SEARCH } from "@/lib/nav";
import { AddPartDialog } from "@/ops/add-part/AddPartDialog";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { PartDetail } from "./detail/PartDetail";
import { PartsOverview } from "./list/PartsOverview";
import { PartsTable } from "./list/PartsTable";
import { PartsTree } from "./list/PartsTree";

/** 表示方法（タブ）。一覧とツリーは詳細を横の欄に、表はモーダルに出す */
const VIEWS = ["overview", "tree", "table"] as const;

/** 一覧の絞り込み（検索の文字列とグループ） */
type Filters = { search: string; group: string };

/** 一覧・ツリー・表のタブ。表はモーダル、一覧とツリーはサイドパネルに詳細を出す。選択と表示方法は URL に載せる。 */
export function ComponentsPage() {
  const t = useT().components;
  const design = useDesign();
  const route = useDetailRoute(paths.components(), VIEWS);
  const [filters, setFilters] = useState<Filters>({ search: "", group: "" });
  const overview = route.view === "overview";
  const close = () => route.select(null);
  return (
    <>
      <ListDetailPage
        design={design}
        route={route}
        wideList={overview}
        hint={t.selectHint}
        header={
          <ComponentsHeader
            design={design.data}
            filters={filters}
            onFilters={setFilters}
            onAdd={() => route.setAdding(true)}
          />
        }
        tabs={<ComponentsTabs view={route.view} />}
        list={(d) => <ComponentsList design={d} filters={filters} route={route} />}
        detail={(d, id) => {
          const item = d.parts.find((p) => p.id === id);
          return item ? (
            <PartDetail
              key={item.id}
              design={d}
              item={item}
              onSelect={route.select}
              onClose={overview ? close : undefined}
            />
          ) : null;
        }}
      />
      {design.data && route.adding && (
        <AddPartDialog
          design={design.data}
          open
          onOpenChange={route.setAdding}
          initialGroup={filters.group || null}
          onDone={(id) => {
            setFilters({ search: "", group: "" });
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
  onFilters,
  onAdd,
}: {
  design?: DesignView;
  filters: Filters;
  onFilters: (filters: Filters) => void;
  onAdd: () => void;
}) {
  const t = useT().components;
  return (
    <PageHeader
      eyebrow={t.eyebrow}
      title={t.title}
      subtitle={design ? t.subtitle(design.parts.length, design.defs.systems.length) : undefined}
      actions={
        <>
          <SearchInput
            label={t.searchLabel}
            placeholder={t.searchPlaceholder}
            value={filters.search}
            onChange={(search) => onFilters({ ...filters, search })}
          />
          <NativeSelect
            aria-label={t.groupFilter}
            value={filters.group}
            onChange={(e) => onFilters({ ...filters, group: e.target.value })}
            placeholder={t.allGroups}
            options={(design?.defs.systems ?? []).map((system) => ({
              value: system,
              label: systemTitle(system, design?.defs),
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
      tabs={VIEWS.map((v) => ({ to: `?${SEARCH.view}=${v}`, label: label[v], active: view === v }))}
    />
  );
}

function ComponentsList({
  design,
  filters,
  route,
}: {
  design: DesignView;
  filters: Filters;
  route: DetailRoute;
}) {
  const t = useT();
  const { search, group } = filters;
  const items = useMemo(
    () => filterParts(design.parts, search, group),
    [design.parts, search, group],
  );
  if (!items.length) return <Empty>{t.components.noMatch}</Empty>;
  const props = { items, selected: route.selected, onSelect: route.select };
  if (route.view === "overview") return <PartsOverview design={design} {...props} />;
  if (route.tree)
    return (
      <PartsTree
        key={JSON.stringify([search, group])}
        all={design.parts}
        folderLabel={(id) => nodeLabel(design, id)}
        {...props}
      />
    );
  return <PartsTable design={design} {...props} />;
}

function filterParts(parts: PartRow[], search: string, group: string) {
  return parts.filter(
    (p) =>
      (!group || p.system === group || p.parent === group) &&
      matchesSearch(search, [p.id, p.kind, p.catalog, p.file]),
  );
}
