import { Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useDesign } from "@/api/queries";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { ListDetailPage } from "@/kit/ListDetailPage";
import { NativeSelect } from "@/kit/NativeSelect";
import { PageTabs } from "@/kit/PageTabs";
import { SearchInput } from "@/kit/SearchInput";
import { matchesSearch } from "@/kit/search";
import { type DetailRoute, useDetailRoute } from "@/kit/use-detail-route";
import { groupsOf, nodeName } from "@/lib/design";
import { paths, SEARCH } from "@/lib/nav";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { AddPartDialog } from "./AddPartDialog";
import { PartDetail } from "./PartDetail";
import { PartsOverview } from "./PartsOverview";
import { PartsTable } from "./PartsTable";
import { PartsTree } from "./PartsTree";

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
          const row = d.parts.find((p) => p.id === id);
          return row ? (
            <PartDetail
              key={row.id}
              design={d}
              row={row}
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
          onCreated={(id) => {
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
      subtitle={design ? t.subtitle(design.parts.length, groupsOf(design).length) : undefined}
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
  const parts = useMemo(
    () => filterParts(design.parts, search, group),
    [design.parts, search, group],
  );
  if (!parts.length) return <Empty>{t.components.noMatch}</Empty>;
  const props = { design, parts, selected: route.selected, onSelect: route.select };
  if (route.view === "overview") return <PartsOverview {...props} />;
  if (route.tree) return <PartsTree key={JSON.stringify([search, group])} {...props} />;
  return <PartsTable {...props} />;
}

function filterParts(parts: PartRow[], search: string, group: string) {
  return parts.filter(
    (p) =>
      (!group || p.subsystem === group || p.parent === group) &&
      matchesSearch(search, [p.id, p.kind, p.catalog, p.file]),
  );
}
