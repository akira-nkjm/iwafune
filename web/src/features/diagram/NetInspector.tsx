import { Trash2 } from "lucide-react";
import { useState } from "react";
import { quantityOf } from "@/api/op-values";
import { useApplyOps } from "@/api/ops";
import type { NetView } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/PropertyList";
import { PropertyRow } from "@/kit/PropertyRow";
import { OpDialog } from "@/ops/OpDialog";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";
import { Button } from "@/ui/button";

/** ネットのインスペクタ: 種類・供給元・レール・プロトコル・所属ポート。レールの編集（UpdateNet）と削除（RemoveNet） */
export function NetInspector({ net }: { net: NetView }) {
  const apply = useApplyOps();
  const [removing, setRemoving] = useState(false);
  const tn = useT().diagram.net;
  return (
    <div className="space-y-3 p-4">
      <div className="flex items-center justify-between gap-2">
        <div>
          <h2 className="font-mono font-semibold text-[15px]">{net.id}</h2>
          <p className="text-ink-3 text-xs">
            {net.kind}
            {net.label ? ` · ${net.label}` : ""} · {net.file}
          </p>
        </div>
        <Button size="sm" variant="outline" onClick={() => setRemoving(true)}>
          <Trash2 size={13} />
          {tn.remove}
        </Button>
      </div>
      <PropertyList>
        <PropertyRow label={tn.sourceMaster}>
          <span className="font-mono">
            {[net.source ?? net.master].map((ref) => (ref ? `${ref.part}.${ref.port}` : "—"))}
          </span>
        </PropertyRow>
        {net.kind === "power" && (
          <PropertyRow label={tn.rail}>
            <EditableQuantity
              display={net.rail?.display ?? "—"}
              initial={quantityInputText(net.rail)}
              dimension="Voltage"
              label={`${net.id}.rail`}
              muted={!net.rail}
              onCommit={(q, t) =>
                apply.mutate({
                  ops: [
                    {
                      op: "set_net_field",
                      net: net.id,
                      field: { field: "rail", value: quantityOf(q) },
                    },
                  ],
                  summary: tn.railSummary(net.id, t),
                })
              }
            />
          </PropertyRow>
        )}
        <PropertyRow label={tn.limit}>
          <span>{net.limit?.display ?? "—"}</span>
        </PropertyRow>
        <PropertyRow label={tn.protocol}>
          <span>
            {net.protocol ?? "—"}
            {net.bitrate ? ` · ${net.bitrate.display}` : ""}
          </span>
        </PropertyRow>
      </PropertyList>
      <div>
        <h3 className="mb-1 font-semibold text-[12.5px]">{tn.members(net.members.length)}</h3>
        <ul className="divide-y divide-line-soft rounded-control border border-line">
          {net.members.map((m) => {
            const { part, port } = m;
            return (
              <li
                key={`${m.part}.${m.port}`}
                className="flex items-center justify-between px-3 py-1 text-xs"
              >
                <span className="font-mono">{`${m.part}.${m.port}`}</span>
                <button
                  type="button"
                  className="text-[11px] text-ink-3 hover:text-neg-ink"
                  onClick={() =>
                    part &&
                    port &&
                    apply.mutate({
                      ops: [{ op: "set_port", part, port, net: null }],
                      summary: tn.detachSummary(`${m.part}.${m.port}`, net.id),
                    })
                  }
                >
                  {tn.detach}
                </button>
              </li>
            );
          })}
        </ul>
      </div>
      {removing && (
        <OpDialog
          open
          onOpenChange={(o) => !o && setRemoving(false)}
          title={tn.removeTitle(net.id)}
          description={tn.removeDescription(net.file, net.members.length)}
          ops={[{ op: "remove_net", id: net.id }]}
          summary={tn.removeSummary(net.id)}
          confirmLabel={tn.removeConfirm}
          destructive
        />
      )}
    </div>
  );
}
