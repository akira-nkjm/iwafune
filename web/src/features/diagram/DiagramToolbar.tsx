import { useT } from "@/i18n";
import { Toggle } from "@/kit/Toggle";
import { LAYERS, type Layer } from "./power";

/** 系統図の表示の切り替え: 層（給電・信号・GND を重ねて、どの組み合わせでも）、同種をまとめる */
export function DiagramToolbar({
  layers,
  onLayers,
  merge,
  onMerge,
}: {
  layers: ReadonlySet<Layer>;
  onLayers: (l: ReadonlySet<Layer>) => void;
  merge: boolean;
  onMerge: (v: boolean) => void;
}) {
  const t = useT().diagram.toolbar;
  const flip = (k: Layer) => {
    const next = new Set(layers);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    onLayers(next);
  };
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="mr-0.5 text-[11px] text-ink-3">{t.layers}</span>
      {LAYERS.map((l) => (
        <Toggle key={l} value={layers.has(l)} onChange={() => flip(l)}>
          {t[l]}
        </Toggle>
      ))}
      <span className="mx-1.5 h-5 w-px bg-line" />
      <Toggle value={merge} onChange={onMerge} title={t.mergeTitle}>
        {t.merge}
      </Toggle>
    </div>
  );
}
