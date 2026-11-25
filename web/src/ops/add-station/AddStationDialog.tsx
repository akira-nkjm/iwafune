import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { CommandText } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";

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

/**
 * 地上局の追加（CreateGroundStation）。軌道と地上局のページのボタンと、足りない入力の行（`AddListItem`）から開く。
 * `initialId` は足りない入力が名指しした局の ID（API の `Fix::AddListItem.id`）で、欄の初期値にする。
 */
export function AddStationDialog({
  initialId,
  onOpenChange,
}: {
  initialId?: string | null;
  onOpenChange: (open: boolean) => void;
}) {
  const ts = useT().conditions.stations;
  const [fresh, setFresh] = useState<NewStation>({ ...EMPTY, id: initialId ?? "" });
  const made = useCommandOps(groundAddCommand(fresh));
  const failed = useOpCheck(made.ops).failed;
  const error = made.error;
  const set = (key: keyof NewStation) => (e: { target: { value: string } }) =>
    setFresh((f) => ({ ...f, [key]: e.target.value }));
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
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
  );
}
