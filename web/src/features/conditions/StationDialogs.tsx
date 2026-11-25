import { Pencil, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import type { GroundStationView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailAction } from "@/kit/detail/DetailPane";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { AddStationDialog } from "@/ops/add-station/AddStationDialog";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import { Button } from "@/ui/button";

/** 軌道と地上局のページの「地上局を足す」ボタン（ダイアログは `ops/add-station`。足りない入力の行からも同じものを開く） */
export function AddStationButton() {
  const ts = useT().conditions.stations;
  const [adding, setAdding] = useState(false);
  return (
    <>
      <Button size="sm" onClick={() => setAdding(true)}>
        <Plus size={13} />
        {ts.add}
      </Button>
      {adding && <AddStationDialog onOpenChange={setAdding} />}
    </>
  );
}

/** 地上局 1 つの操作（名前の変更 RenameGroundStation・削除 RemoveGroundStation）。解析が挙げている局は削除できない（API の `in_use`） */
export function StationActions({ station }: { station: GroundStationView }) {
  const t = useT();
  const ts = t.conditions.stations;
  const dialogs = useDialogs<"rename" | "remove">();
  return (
    <>
      <div className="flex items-center gap-1.5">
        <DetailAction
          icon={Pencil}
          label={ts.rename(station.id)}
          onClick={() => dialogs.show("rename")}
        />
        <DetailAction
          icon={Trash2}
          label={station.in_use ? ts.inUse : ts.remove(station.id)}
          onClick={() => dialogs.show("remove")}
        />
      </div>
      {dialogs.current === "rename" && (
        <RenameDialog
          {...dialogs.props}
          from={station.id}
          title={ts.renameTitle(station.id)}
          description={ts.renameDescription}
          label={ts.newId}
          confirmLabel={ts.renameConfirm}
          opsFor={(to) => [{ op: "rename_ground_station", id: station.id, to }]}
          summary={(to) => ts.renameSummary(station.id, to)}
        />
      )}
      {dialogs.current === "remove" && (
        <OpDialog
          {...dialogs.props}
          title={ts.removeTitle(station.id)}
          description={station.in_use ? ts.inUse : ts.removeDescription(station.file)}
          ops={station.in_use ? null : [{ op: "remove_ground_station", id: station.id }]}
          summary={ts.removeSummary(station.id)}
          confirmLabel={ts.removeConfirm}
          destructive
        />
      )}
    </>
  );
}
