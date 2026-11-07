import { useCallback, useState } from "react";
import { quantityOf } from "@/api/op-values";
import type { NetKind, Op, QuantityView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { NativeSelect } from "@/kit/NativeSelect";
import { TextInput } from "@/kit/TextInput";

import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { QuantityInput } from "@/quantity/QuantityInput";

type DiagramView = "power" | "signal";

type End = { part: string; port: string };

/** 未接続のポートどうしを繋いだとき: 新しいネットを作り（CreateNet）、両方のポートをそこに入れる（SetPort） */
export function NewNetDialog({
  view,
  a,
  b,
  onClose,
}: {
  view: DiagramView;
  a: End;
  b: End;
  onClose: () => void;
}) {
  const t = useT();
  const tn = t.diagram.newNet;
  const [id, setId] = useState(view === "power" ? `P_${a.part}` : `BUS_${a.part}`);
  const [kind, setKind] = useState<NetKind>(view === "power" ? "power" : "data");
  const [source, setSource] = useState(`${a.part}.${a.port}`);
  const [rail, setRail] = useState<{ text: string; ok: boolean; q: QuantityView | null }>({
    text: "",
    ok: false,
    q: null,
  });
  const [protocol, setProtocol] = useState("");
  const onRail = useCallback(
    (text: string, ok: boolean, q: QuantityView | null) => setRail({ text, ok, q }),
    [],
  );

  const railOk = kind !== "power" || rail.text.trim() === "" || rail.ok;
  const ops: Op[] | null =
    id && railOk
      ? [
          {
            op: "create_net",
            net: {
              id,
              kind,
              label: null,
              source: kind === "power" ? source : null,
              rail: kind === "power" && rail.q ? quantityOf(rail.q) : null,
              protocol: kind !== "power" && protocol ? protocol : null,
            },
          },
          { op: "set_port", part: a.part, port: a.port, net: id },
          { op: "set_port", part: b.part, port: b.port, net: id },
        ]
      : null;
  // ID の規則と重複はサーバー（dry_run）が検査する
  const error = useOpCheck(ops).error;
  const ends = [`${a.part}.${a.port}`, `${b.part}.${b.port}`];
  return (
    <OpDialog
      open
      onOpenChange={(o) => !o && onClose()}
      title={tn.title}
      description={tn.description(ends[0] ?? "", ends[1] ?? "")}
      ops={ops}
      summary={tn.summary(id)}
      confirmLabel={tn.confirm}
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label={tn.id} htmlFor="net-id" error={error}>
          <TextInput
            id="net-id"
            mono
            value={id}
            invalid={!!error}
            onChange={(e) => setId(e.target.value)}
          />
        </Field>
        <Field label={tn.kind} htmlFor="net-kind">
          <NativeSelect
            id="net-kind"
            value={kind}
            onChange={(e) => setKind(e.target.value as NetKind)}
            options={[
              { value: "power", label: tn.power },
              { value: "data", label: tn.data },
              { value: "rf", label: tn.rf },
            ]}
          />
        </Field>
      </div>
      {kind === "power" ? (
        <div className="grid grid-cols-2 gap-3">
          <Field label={tn.source} htmlFor="net-source">
            <NativeSelect
              id="net-source"
              value={source}
              onChange={(e) => setSource(e.target.value)}
              options={ends.map((e) => ({ value: e, label: e }))}
            />
          </Field>
          <Field label={tn.rail}>
            <QuantityInput
              initial=""
              dimension="Voltage"
              label={tn.railLabel}
              onChangeValid={onRail}
            />
          </Field>
        </div>
      ) : (
        <Field label={tn.protocol} htmlFor="net-proto">
          <TextInput
            id="net-proto"
            value={protocol}
            placeholder="CAN"
            onChange={(e) => setProtocol(e.target.value)}
          />
        </Field>
      )}
    </OpDialog>
  );
}
