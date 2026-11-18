import { useState } from "react";
import { useEnumLabel } from "@/api/labels";
import { useSetText } from "@/api/ops";
import type { ModeSequenceView, ModeStartView, ModeTable } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Button } from "@/ui/button";

/** 解析に使う並びを選ぶ設定のパス（`fune setting set analysis.power.active_sequence <名前>` と同じ） */
const ACTIVE_SEQUENCE = "analysis.power.active_sequence";

/**
 * 電源のモードの並び（名前付き）の一覧。並びを切り替えて区間と先頭の位置を見られ、「解析に使う」で
 * 電力収支が回す並びを選べる（`analysis.power.active_sequence`）。並びが 1 つも無ければ何も出さない。
 * 区間の編集は `fune sequence set`（GUI では見るだけ）。
 */
export function SequencePanel({ table }: { table: ModeTable }) {
  const t = useT().modes.sequences;
  const enumLabel = useEnumLabel();
  const setText = useSetText();
  const [picked, setPicked] = useState<string | null>(null);
  const { sequences, active_sequence: active } = table;
  const shown =
    sequences.find((s) => s.name === picked) ??
    sequences.find((s) => s.name === active) ??
    sequences[0];
  if (!shown) return null;
  const inUse = shown.name === active;
  return (
    <section
      aria-label={t.title}
      className="mx-6 mt-4 rounded-card border border-line bg-panel px-4 py-3"
    >
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <h2 className="font-semibold text-sm">{t.title}</h2>
        <NativeSelect
          aria-label={t.pick}
          size="sm"
          options={sequences.map((s) => ({
            value: s.name,
            label: s.name === active ? `${s.name} (${t.inUse})` : s.name,
          }))}
          value={shown.name}
          onChange={(e) => setPicked(e.target.value)}
        />
        <p className="text-ink-3 text-xs">{t.hint}</p>
        <Button
          variant="outline"
          className="ml-auto"
          disabled={inUse || setText.isPending}
          onClick={() =>
            setText.mutate({
              path: ACTIVE_SEQUENCE,
              value: shown.name,
              summary: t.chooseSummary(shown.name),
            })
          }
        >
          {inUse ? t.inUse : t.use}
        </Button>
      </div>
      <SequenceDetail sequence={shown} enumLabel={enumLabel} />
    </section>
  );
}

/** 並び 1 つの先頭の位置と区間（モード → 長さの順） */
function SequenceDetail({
  sequence,
  enumLabel,
}: {
  sequence: ModeSequenceView;
  enumLabel: (ty: string, value: string) => string;
}) {
  const t = useT().modes;
  return (
    <div className="mt-2 space-y-1.5 text-xs">
      <p className="text-ink-3">
        {t.sequences.anchor}:{" "}
        {anchorText(sequence.anchor, enumLabel, t.timing.atEvent) || t.sequences.anchorNone}
        {sequence.legacy && ` · ${t.sequences.legacy}`}
        {` · ${t.sequences.segmentCount(sequence.segments.length)}`}
      </p>
      <ol className="flex flex-wrap items-center gap-1.5">
        {sequence.segments.map((s, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: 並びは同じモードが何度も出るので、順番が区間の同一性
          <li key={i} className="rounded border border-line bg-sunken px-2 py-0.5">
            <span className="font-mono">{s.mode}</span>{" "}
            <span className="num text-ink-3">{s.duration.display}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}

/** 「食明け +5 min」のような先頭の位置（無ければ空） */
function anchorText(
  anchor: ModeStartView | null,
  enumLabel: (ty: string, value: string) => string,
  atEvent: (event: string, offset: string) => string,
): string {
  if (!anchor) return "";
  return atEvent(enumLabel("start_from", anchor.from), anchor.offset?.display ?? "");
}
