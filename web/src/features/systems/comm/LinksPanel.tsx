import { Pencil, Trash2 } from "lucide-react";
import type { DesignView, LinkView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailAction } from "@/kit/detail/DetailPane";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { Empty } from "@/kit/feedback/Empty";
import { Panel } from "@/kit/panel/Panel";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { LinkDialog } from "@/ops/link/LinkDialog";
import { OpDialog } from "@/ops/OpDialog";

/** 通信のページの回線の一覧。1 本ずつ開いて直す（`LinkDialog`）・消す（RemoveCommLink）。足すのはページ上部のボタン（`AddLinkButton`） */
export function LinksPanel({ design }: { design: DesignView }) {
  const tl = useT().conditions.links;
  if (design.links.length === 0) return <Empty>{tl.noLinks}</Empty>;
  return (
    <Panel pad={false}>
      <div className="border-line border-b px-4 py-2.5">
        <h2 className="font-medium text-ink text-md">{tl.panelTitle}</h2>
        <p className="text-ink-3 text-xs">{tl.panelCaption(design.links[0]?.file ?? "")}</p>
      </div>
      <DataTable
        head={
          <>
            <Th>{tl.id}</Th>
            <Th>{tl.direction}</Th>
            <Th>{tl.part}</Th>
            <Th>{tl.band}</Th>
            <Th />
          </>
        }
      >
        {design.links.map((l) => (
          <LinkRow key={l.id} link={l} design={design} />
        ))}
      </DataTable>
    </Panel>
  );
}

/** 回線 1 本の行と、その編集・削除のダイアログ */
function LinkRow({ link, design }: { link: LinkView; design: DesignView }) {
  const tl = useT().conditions.links;
  const dialogs = useDialogs<"edit" | "remove">();
  return (
    <tr>
      <Td mono>{link.id}</Td>
      <Td mono>{link.direction}</Td>
      <Td mono>{link.part}</Td>
      <Td mono>{link.band}</Td>
      <Td>
        <div className="flex items-center justify-end gap-1.5">
          <DetailAction
            icon={Pencil}
            label={tl.edit(link.id)}
            onClick={() => dialogs.show("edit")}
          />
          <DetailAction
            icon={Trash2}
            label={tl.remove(link.id)}
            onClick={() => dialogs.show("remove")}
          />
        </div>
        {dialogs.current === "edit" && (
          <LinkDialog design={design} editing={link} onOpenChange={dialogs.props.onOpenChange} />
        )}
        {dialogs.current === "remove" && (
          <OpDialog
            {...dialogs.props}
            title={tl.removeTitle(link.id)}
            description={tl.removeDescription(link.file)}
            ops={[{ op: "remove_comm_link", id: link.id }]}
            summary={tl.removeSummary(link.id)}
            confirmLabel={tl.removeConfirm}
            destructive
          />
        )}
      </Td>
    </tr>
  );
}
