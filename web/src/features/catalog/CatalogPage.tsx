import { Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useDesign } from "@/api/queries";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { SearchInput } from "@/kit/forms/SearchInput";
import { matchesSearch } from "@/kit/forms/search";
import { ListDetailPage } from "@/kit/list-detail/ListDetailPage";
import { type DetailRoute, useDetailRoute } from "@/kit/list-detail/use-detail-route";
import { PageTabs } from "@/kit/selection/PageTabs";
import { CATALOG_GROUPINGS, type CatalogGrouping } from "@/kit/tree/tree-nodes";
import { kindLabel } from "@/lib/design";
import { paths, SEARCH } from "@/lib/nav";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { CatalogDetail } from "./CatalogDetail";
import { CatalogTable } from "./CatalogTable";
import { CatalogTree } from "./CatalogTree";
import { CreateCatalogDialog } from "./CreateCatalogDialog";
import { itemLabel } from "./catalog";
import { SharedCatalogsButton } from "./SharedCatalogsButton";
import { neededCount, SharedCatalogsDialog, sharedOf } from "./SharedCatalogsDialog";
import { useCatalogGrouping } from "./use-catalog-grouping";

/** 表示方法（タブ）。表はモーダル、ツリーはサイドパネルに詳細を出す。タブを足すときはここと `CatalogTabs` の見出しに足す */
const VIEWS = ["tree", "table"] as const;

/** 一覧の絞り込み（検索の文字列・カタログ名・種別。空は絞らない） */
type Filters = { search: string; catalog: string; kind: string };
const NO_FILTERS: Filters = { search: "", catalog: "", kind: "" };

/** カタログも同じ URL 選択で、表ではモーダル・ツリーではサイドパネルを開く。共有カタログの lock の状態は見出しのボタンからダイアログで見る。 */
export function CatalogPage() {
  const t = useT();
  const design = useDesign();
  const route = useDetailRoute(paths.catalog(), VIEWS);
  const [filters, setFilters] = useState<Filters>(NO_FILTERS);
  const [grouping, setGrouping] = useCatalogGrouping();
  /** 共有カタログのダイアログを開いているか（ページの中だけの状態。URL には出さない） */
  const [sharedOpen, setSharedOpen] = useState(false);
  return (
    <>
      <ListDetailPage
        design={design}
        route={route}
        hint={t.catalog.select}
        header={
          <CatalogHeader
            design={design.data}
            filters={filters}
            onFilters={setFilters}
            tree={route.tree}
            grouping={grouping}
            onGrouping={setGrouping}
            onAdd={() => route.setAdding(true)}
            onShared={() => setSharedOpen(true)}
          />
        }
        tabs={<CatalogTabs view={route.view} grouping={grouping} />}
        list={(d) => <CatalogList design={d} filters={filters} grouping={grouping} route={route} />}
        detail={(d, id) => {
          const item = d.catalog.find((c) => c.id === id);
          return item ? (
            <CatalogDetail key={item.id} design={d} item={item} onSelect={route.select} />
          ) : null;
        }}
      />
      {design.data && sharedOpen && (
        <SharedCatalogsDialog
          catalogs={design.data.project.catalogs}
          open
          onOpenChange={setSharedOpen}
        />
      )}
      {design.data && route.adding && (
        <CreateCatalogDialog
          design={design.data}
          open
          onOpenChange={route.setAdding}
          onDone={(id) => {
            setFilters(NO_FILTERS);
            route.select(id);
          }}
        />
      )}
    </>
  );
}

function CatalogHeader({
  design,
  filters,
  onFilters,
  tree,
  grouping,
  onGrouping,
  onAdd,
  onShared,
}: {
  design?: DesignView;
  filters: Filters;
  onFilters: (filters: Filters) => void;
  /** ツリーの表示か（分け方の選択はツリーだけに出す） */
  tree: boolean;
  grouping: CatalogGrouping;
  onGrouping: (grouping: CatalogGrouping) => void;
  onAdd: () => void;
  /** 共有カタログのダイアログを開く */
  onShared: () => void;
}) {
  const t = useT();
  // 種別の選択肢は、カタログに出てくる種別だけ（表示名で並べる）
  const kinds = useMemo(() => {
    if (!design) return [];
    const ids = [...new Set(design.catalog.map((c) => c.kind))];
    return ids
      .map((value) => ({ value, label: kindLabel(design.defs, value) }))
      .sort((a, b) => a.label.localeCompare(b.label));
  }, [design]);
  return (
    <PageHeader
      eyebrow={t.nav.sections.design}
      title={t.catalog.title}
      subtitle={
        design
          ? t.catalog.subtitle(
              design.catalog.length,
              design.catalog.filter((c) => c.editable).length,
            )
          : undefined
      }
      actions={
        <>
          <SearchInput
            label={t.catalog.search}
            value={filters.search}
            onChange={(search) => onFilters({ ...filters, search })}
          />
          <NativeSelect
            aria-label={t.catalog.catalogFilter}
            value={filters.catalog}
            onChange={(e) => onFilters({ ...filters, catalog: e.target.value })}
            placeholder={t.catalog.allCatalogs}
            options={(design?.project.catalogs ?? []).map((c) => ({
              value: c.name,
              label: c.name,
            }))}
          />
          <NativeSelect
            aria-label={t.catalog.kindFilter}
            value={filters.kind}
            onChange={(e) => onFilters({ ...filters, kind: e.target.value })}
            placeholder={t.catalog.allKinds}
            options={kinds}
          />
          {tree && (
            <NativeSelect
              aria-label={t.catalog.groupBy}
              value={grouping}
              onChange={(e) => onGrouping(e.target.value as CatalogGrouping)}
              options={CATALOG_GROUPINGS.map((g) => ({ value: g, label: t.catalog.groupings[g] }))}
            />
          )}
          <SharedCatalogsButton
            total={sharedOf(design?.project.catalogs ?? []).length}
            needed={neededCount(design?.project.catalogs ?? [])}
            onClick={onShared}
          />
          <Button onClick={onAdd} disabled={!design}>
            <Plus size={14} />
            {t.catalog.add}
          </Button>
        </>
      }
    />
  );
}

/** タブを切り替えてもツリーの分け方（`?group=`）は残す */
function CatalogTabs({ view, grouping }: { view: string; grouping: CatalogGrouping }) {
  const t = useT().catalog;
  const label = { tree: t.tabTree, table: t.tabTable };
  const to = (v: string) =>
    `?${new URLSearchParams({ [SEARCH.view]: v, [SEARCH.group]: grouping })}`;
  return <PageTabs tabs={VIEWS.map((v) => ({ to: to(v), label: label[v], active: view === v }))} />;
}

function CatalogList({
  design,
  filters,
  grouping,
  route,
}: {
  design: DesignView;
  filters: Filters;
  grouping: CatalogGrouping;
  route: DetailRoute;
}) {
  const t = useT();
  const { search, catalog, kind } = filters;
  const items = useMemo(
    () =>
      design.catalog.filter(
        (c) =>
          (!catalog || c.scope === catalog) &&
          (!kind || c.kind === kind) &&
          matchesSearch(search, [c.id, c.kind, kindLabel(design.defs, c.kind), itemLabel(c)]),
      ),
    [design.catalog, design.defs, search, catalog, kind],
  );
  if (!items.length) return <Empty>{t.catalog.empty}</Empty>;
  const props = { design, items, selected: route.selected, onSelect: route.select };
  if (route.tree)
    return (
      <CatalogTree
        key={JSON.stringify([search, catalog, kind])}
        all={design.catalog}
        grouping={grouping}
        {...props}
      />
    );
  return <CatalogTable {...props} />;
}
