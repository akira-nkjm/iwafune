import { Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useDesign } from "@/api/queries";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { ListDetailPage } from "@/kit/ListDetailPage";
import { SearchInput } from "@/kit/SearchInput";
import { matchesSearch } from "@/kit/search";
import { type DetailRoute, useDetailRoute } from "@/kit/use-detail-route";
import { ViewSwitch } from "@/kit/ViewSwitch";
import { paths } from "@/lib/nav";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { CatalogDetail } from "./CatalogDetail";
import { CatalogTable } from "./CatalogTable";
import { CatalogTree } from "./CatalogTree";
import { CreateCatalogDialog } from "./CreateCatalogDialog";
import { itemLabel } from "./catalog";

/** カタログも同じ URL 選択で、表ではモーダル・ツリーではサイドパネルを開く。 */
export function CatalogPage() {
  const t = useT();
  const design = useDesign();
  const route = useDetailRoute(paths.catalog());
  const [search, setSearch] = useState("");
  return (
    <>
      <ListDetailPage
        design={design}
        route={route}
        hint={t.catalog.select}
        header={
          <CatalogHeader
            design={design.data}
            view={route.view}
            onView={route.setView}
            search={search}
            onSearch={setSearch}
            onAdd={() => route.setAdding(true)}
          />
        }
        list={(d) => <CatalogList design={d} search={search} route={route} />}
        detail={(d, id) => {
          const item = d.catalog.find((c) => c.id === id);
          return item ? (
            <CatalogDetail key={item.id} design={d} item={item} onSelect={route.select} />
          ) : null;
        }}
      />
      {design.data && route.adding && (
        <CreateCatalogDialog
          design={design.data}
          open
          onOpenChange={route.setAdding}
          onDone={(id) => {
            setSearch("");
            route.select(id);
          }}
        />
      )}
    </>
  );
}

function CatalogHeader({
  design,
  view,
  onView,
  search,
  onSearch,
  onAdd,
}: {
  design?: DesignView;
  /** 表示方法（`useDetailRoute` の `view`・`setView`） */
  view: string;
  onView: (view: string) => void;
  search: string;
  onSearch: (value: string) => void;
  onAdd: () => void;
}) {
  const t = useT();
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
          <ViewSwitch value={view} onChange={onView} />
          <SearchInput label={t.catalog.search} value={search} onChange={onSearch} />
          <Button onClick={onAdd} disabled={!design}>
            <Plus size={14} />
            {t.catalog.add}
          </Button>
        </>
      }
    />
  );
}

function CatalogList({
  design,
  search,
  route,
}: {
  design: DesignView;
  search: string;
  route: DetailRoute;
}) {
  const t = useT();
  const items = useMemo(
    () => design.catalog.filter((c) => matchesSearch(search, [c.id, c.kind, itemLabel(c)])),
    [design.catalog, search],
  );
  if (!items.length) return <Empty>{t.catalog.empty}</Empty>;
  if (route.tree)
    return (
      <CatalogTree
        key={search}
        all={design.catalog}
        items={items}
        selected={route.selected}
        onSelect={route.select}
      />
    );
  return <CatalogTable items={items} selected={route.selected} onSelect={route.select} />;
}
