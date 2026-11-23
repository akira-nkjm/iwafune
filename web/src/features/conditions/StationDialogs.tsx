import { Pencil, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { CommandText, GroundStationView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailAction } from "@/kit/detail/DetailPane";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";

/** 地上局を足す欄（打った文字列のまま。量の読み方は service の `ground_add`） */
interface NewStation {
  id: string;
  label: string;
  latitude: string;
  longitude: string;
  altitude: string;
  minElevation: string;
}

const EMPTY: NewStation = {
  id: "",
  label: "",
  latitude: "",
  longitude: "",
  altitude: "",
  minElevation: "",
};

/** `fune ground add` と同じ引数（`POST /ops/from_command` の `ground_add`）。ID・緯度・経度が空の間は作らない（位置が無い局は解析に使えない） */
function groundAddCommand(s: NewStation): CommandText | null {
  if (!s.id.trim() || !s.latitude.trim() || !s.longitude.trim()) return null;
  return {
    command: "ground_add",
    id: s.id,
    label: s.label,
    latitude: s.latitude,
    longitude: s.longitude,
    altitude: s.altitude,
    min_elevation: s.minElevation,
  };
}

/** 軌道と地上局のページの「地上局を足す」ボタンとダイアログ（CreateGroundStation） */
export function AddStationButton() {
  const t = useT();
  const ts = t.conditions.stations;
  const [adding, setAdding] = useState(false);
  const [fresh, setFresh] = useState<NewStation>(EMPTY);
  const made = useCommandOps(adding ? groundAddCommand(fresh) : null);
  const failed = useOpCheck(adding ? made.ops : null).failed;
  const error = made.error;
  const set = (key: keyof NewStation) => (e: { target: { value: string } }) =>
    setFresh((f) => ({ ...f, [key]: e.target.value }));
  return (
    <>
      <Button size="sm" onClick={() => setAdding(true)}>
        <Plus size={13} />
        {ts.add}
      </Button>
      {adding && (
        <OpDialog
          open
          onOpenChange={(o) => {
            if (!o) {
              setAdding(false);
              setFresh(EMPTY);
            }
          }}
          title={ts.addTitle}
          description={ts.addDescription}
          ops={made.ops}
          summary={ts.addSummary(fresh.id.trim())}
          confirmLabel={ts.addConfirm}
        >
          <div className="grid grid-cols-2 gap-3">
            <Field label="ID" htmlFor="gs-id" error={error}>
              <TextInput
                id="gs-id"
                mono
                value={fresh.id}
                invalid={failed || !!error}
                placeholder="SVALBARD_GS"
                onChange={set("id")}
              />
            </Field>
            <Field label={ts.label} htmlFor="gs-label">
              <TextInput id="gs-label" value={fresh.label} onChange={set("label")} />
            </Field>
            <Field label={ts.latitude} htmlFor="gs-lat" hint={ts.quantityHint}>
              <TextInput
                id="gs-lat"
                mono
                value={fresh.latitude}
                placeholder="78.2 deg"
                onChange={set("latitude")}
              />
            </Field>
            <Field label={ts.longitude} htmlFor="gs-lon">
              <TextInput
                id="gs-lon"
                mono
                value={fresh.longitude}
                placeholder="15.4 deg"
                onChange={set("longitude")}
              />
            </Field>
            <Field label={ts.altitude} htmlFor="gs-alt">
              <TextInput
                id="gs-alt"
                mono
                value={fresh.altitude}
                placeholder="450 m"
                onChange={set("altitude")}
              />
            </Field>
            <Field label={ts.minElevation} htmlFor="gs-min-el">
              <TextInput
                id="gs-min-el"
                mono
                value={fresh.minElevation}
                placeholder="5 deg"
                onChange={set("minElevation")}
              />
            </Field>
          </div>
        </OpDialog>
      )}
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
