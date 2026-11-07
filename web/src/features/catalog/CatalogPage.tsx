import { Plus, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { useParams } from "react-router";
import { useDesign } from "@/api/queries";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailLayout, DetailLoading } from "@/kit/DetailLayout";
import { Empty } from "@/kit/Empty";
import { TextInput } from "@/kit/TextInput";
import { useDetailRoute } from "@/kit/use-detail-route";
import { ViewSwitch } from "@/kit/ViewSwitch";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { CatalogDetail } from "./CatalogDetail";
import { CatalogTable } from "./CatalogTable";
import { CatalogTree } from "./CatalogTree";
import { CreateCatalogDialog } from "./CreateCatalogDialog";
import { itemLabel } from "./catalog";

type Selection = { tree: boolean; selected: string | null; onSelect: (id: string | null) => void };

/** カタログも同じ URL 選択で、表ではモーダル・ツリーではサイドパネルを開く。 */
export function CatalogPage() {
  const design = useDesign();
  const route = useDetailRoute("/design/catalog");
  const { itemId } = useParams();
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const selection = {
    tree: route.tree,
    selected: itemId ? decodeURIComponent(itemId) : null,
    onSelect: route.select,
  };
  return (
    <>
      <CatalogHeader
        design={design.data}
        search={search}
        onSearch={setSearch}
        onAdd={() => setCreating(true)}
      />
      <CatalogContent design={design} search={search} selection={selection} />
      {design.data && creating && (
        <CreateCatalogDialog
          design={design.data}
          open
          onOpenChange={setCreating}
          onCreated={(id) => {
            setCreating(false);
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
  search,
  onSearch,
  onAdd,
}: {
  design?: DesignView;
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
          <ViewSwitch />
          <div className="relative">
            <Search size={13} className="absolute top-2.5 left-2.5 text-ink-4" />
            <TextInput
              aria-label={t.catalog.search}
              placeholder={t.catalog.search}
              className="w-52 pl-7"
              value={search}
              onChange={(e) => onSearch(e.target.value)}
            />
          </div>
          <Button onClick={onAdd} disabled={!design}>
            <Plus size={14} />
            {t.catalog.add}
          </Button>
        </>
      }
    />
  );
}

function CatalogContent({
  design,
  search,
  selection,
}: {
  design: ReturnType<typeof useDesign>;
  search: string;
  selection: Selection;
}) {
  const t = useT();
  const d = design.data;
  if (design.isError) return <Empty>{t.common.cannotLoadDesign(design.error.message)}</Empty>;
  if (!d) return <DetailLoading tree={selection.tree} />;
  const current = d.catalog.find((c) => c.id === selection.selected);
  return (
    <DetailLayout
      tree={selection.tree}
      title={current?.id ?? t.catalog.title}
      hint={t.catalog.select}
      onClose={() => selection.onSelect(null)}
      list={<CatalogList design={d} search={search} selection={selection} />}
      detail={
        current ? (
          <CatalogDetail key={current.id} design={d} item={current} onSelect={selection.onSelect} />
        ) : null
      }
    />
  );
}

function CatalogList({
  design,
  search,
  selection,
}: {
  design: DesignView;
  search: string;
  selection: Selection;
}) {
  const t = useT();
  const items = useMemo(() => {
    const s = search.trim().toLowerCase();
    return design.catalog.filter(
      (c) => !s || [c.id, c.kind, itemLabel(c)].some((value) => value.toLowerCase().includes(s)),
    );
  }, [design.catalog, search]);
  if (!items.length) return <Empty>{t.catalog.empty}</Empty>;
  if (selection.tree)
    return (
      <CatalogTree
        key={search}
        all={design.catalog}
        items={items}
        selected={selection.selected}
        onSelect={selection.onSelect}
      />
    );
  return <CatalogTable items={items} selected={selection.selected} onSelect={selection.onSelect} />;
}
