import { Pencil, Plus } from "lucide-react";
import { useState } from "react";
import { useEnumLabel } from "@/api/labels";
import { useDesign } from "@/api/queries";
import type { EvidenceSource, SourceView } from "@/api/types";
import { useT } from "@/i18n";
import { ExternalLink } from "@/kit/ExternalLink";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { SourceDialog } from "./SourceDialog";

/** 編集のダイアログの対象（新しい出典は null） */
type Editing = { source: EvidenceSource | null } | null;

/** プロジェクト / 出典: sources/ の一覧。参照先が Web の URL ならリンクで開け、各行から書き直せる */
export function SourcesPage() {
  const t = useT();
  const m = t.sources;
  const design = useDesign();
  const [editing, setEditing] = useState<Editing>(null);
  return (
    <>
      <PageHeader
        eyebrow={t.nav.sections.project}
        title={m.title}
        subtitle={design.data ? m.subtitle(design.data.sources.length) : undefined}
        actions={
          <Button onClick={() => setEditing({ source: null })} disabled={!design.data}>
            <Plus size={14} />
            {m.add}
          </Button>
        }
      />
      <PageBody>
        <QueryState query={design} skeleton={<SkeletonPanel lines={5} />}>
          {(d) =>
            d.sources.length === 0 ? (
              <Empty>{m.empty}</Empty>
            ) : (
              <SourcesTable sources={d.sources} onEdit={(source) => setEditing({ source })} />
            )
          }
        </QueryState>
      </PageBody>
      {editing && (
        <SourceDialog
          key={editing.source?.id ?? ""}
          source={editing.source}
          open
          onOpenChange={(open) => {
            if (!open) setEditing(null);
          }}
        />
      )}
    </>
  );
}

/** 出典の表。SHA-256 は先頭だけを出し、全体はツールチップ */
export function SourcesTable({
  sources,
  onEdit,
}: {
  sources: SourceView[];
  onEdit: (source: SourceView) => void;
}) {
  const m = useT().sources;
  const label = useEnumLabel();
  return (
    <Panel pad={false}>
      <DataTable
        head={
          <>
            <Th>{m.id}</Th>
            <Th>{m.kind}</Th>
            <Th>{m.label}</Th>
            <Th>{m.ref}</Th>
            <Th>{m.revision}</Th>
            <Th>{m.retrieved}</Th>
            <Th>{m.distribution}</Th>
            <Th>{m.sha256}</Th>
            <Th>{m.date}</Th>
            <Th>{m.by}</Th>
            <Th>{m.file}</Th>
            <Th>
              <span className="sr-only">{m.edit}</span>
            </Th>
          </>
        }
      >
        {sources.map((s) => (
          <tr key={s.id}>
            <Td mono>{s.id}</Td>
            <Td>{label("source_kind", s.kind)}</Td>
            <Td>{s.label ?? "—"}</Td>
            <Td className="max-w-[280px]">
              {s.link ? (
                <ExternalLink href={s.link} label={m.open} />
              ) : (
                <span className="font-mono text-[12px] text-ink-2">{s.ref ?? "—"}</span>
              )}
            </Td>
            <Td>{s.revision ?? "—"}</Td>
            <Td mono>{s.retrieved ?? "—"}</Td>
            <Td>{s.distribution ? label("distribution", s.distribution) : "—"}</Td>
            <Td mono className="text-ink-3">
              {s.sha256 ? (
                <span className="block max-w-[96px] truncate" title={s.sha256}>
                  {s.sha256}
                </span>
              ) : (
                "—"
              )}
            </Td>
            <Td mono>{s.date ?? "—"}</Td>
            <Td>{s.by ?? "—"}</Td>
            <Td mono className="text-ink-4">
              {s.file}
            </Td>
            <Td>
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={`${m.edit}: ${s.id}`}
                onClick={() => onEdit(s)}
              >
                <Pencil />
              </Button>
            </Td>
          </tr>
        ))}
      </DataTable>
    </Panel>
  );
}
