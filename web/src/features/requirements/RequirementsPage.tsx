import { Plus } from "lucide-react";
import { useState } from "react";
import { useParams } from "react-router";
import { useDesign } from "@/api/queries";
import type { RequirementText } from "@/api/types";
import { useT } from "@/i18n";
import { PageTabs } from "@/kit/selection/PageTabs";
import { paths } from "@/lib/nav";
import { OpDialog } from "@/ops/OpDialog";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { ComplianceTab } from "./ComplianceTab";
import { RequirementDialog } from "./RequirementDialog";
import { RequirementsTable } from "./RequirementsTable";
import { emptyRequirementText } from "./requirement-form";

/** 開いているダイアログ（追加・編集・削除） */
type Open =
  | { kind: "add" }
  | { kind: "edit"; text: RequirementText }
  | { kind: "remove"; id: string }
  | null;

/**
 * 設計 / 要求: 要求適合表（既定のタブ）と、requirements/ の一覧・追加・編集・削除。
 * 編集は `fune requirement add|set|rm` と同じ引数を service に送る。判定と集計は service（`GET /requirements/compliance`）
 */
export function RequirementsPage() {
  const { tab } = useParams();
  const t = useT().requirements;
  const design = useDesign();
  const [open, setOpen] = useState<Open>(null);
  const close = (o: boolean) => !o && setOpen(null);
  const list = tab === "list";
  const d = design.data;
  return (
    <>
      <PageHeader
        eyebrow={t.eyebrow}
        title={t.title}
        subtitle={t.subtitle}
        actions={
          <Button variant="outline" disabled={!d} onClick={() => setOpen({ kind: "add" })}>
            <Plus size={14} />
            {t.add}
          </Button>
        }
      />
      <PageTabs
        tabs={[
          { to: paths.requirements(), label: t.tabCompliance },
          { to: paths.requirements("list"), label: t.tabList, count: d?.requirements.length },
        ]}
      />
      <PageBody>
        {list ? <RequirementsTable design={design} onOpen={setOpen} /> : <ComplianceTab />}
      </PageBody>
      {d && open?.kind === "add" && (
        <RequirementDialog
          design={d}
          initial={emptyRequirementText()}
          adding
          onOpenChange={close}
        />
      )}
      {d && open?.kind === "edit" && (
        <RequirementDialog design={d} initial={open.text} adding={false} onOpenChange={close} />
      )}
      {open?.kind === "remove" && (
        <OpDialog
          open
          destructive
          onOpenChange={close}
          title={t.removeDialog.title(open.id)}
          description={t.removeDialog.description}
          ops={[{ op: "remove_requirement", id: open.id }]}
          summary={t.removeDialog.summary(open.id)}
          confirmLabel={t.removeDialog.confirm}
        />
      )}
    </>
  );
}
