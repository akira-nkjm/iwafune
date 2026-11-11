import { useState } from "react";
import { useApplyOps, useCommandOps } from "@/api/ops";
import type { CommandText, DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { Formatted } from "@/quantity/Formatted";
import { numbersInputText, vectorInputText } from "@/quantity/format";
import { Button } from "@/ui/button";

/** 姿勢の欄の書き方の例（w x y z） */
const QUATERNION_EXAMPLE = "1 0 0 0";

/**
 * 配置: 取付先と取付点、取付点に対する位置・姿勢（Place）。位置と姿勢は `fune part place --position "0 0 12 mm"
 * --orientation "1 0 0 0"` と同じ文字列で打ち、そのまま `POST /ops/from_command` に送る（読むのは service）。
 * 位置の初期値はファイルに書かれた値を書かれた単位（`authored_position`）で見せる（換算しない）。
 * 機体座標は取付をたどった導出値で、表示だけ。
 */
export function PlacementSection({ design, part }: { design: DesignView; part: PartRow }) {
  const apply = useApplyOps();
  const t = useT().components.placement;
  const m = part.mount;
  // 位置は書かれたものだけを書かれた単位で見せる（書かれていなければ空。空の欄は CLI で旗を書かないのと同じで、書かない）
  const [initial] = useState(() => ({
    on: m?.on ?? "",
    point: m?.point ?? "",
    position: m?.authored_position ? vectorInputText(m.authored_position) : "",
    orientation: m ? numbersInputText(m.orientation) : "",
  }));
  const [mountOn, setMountOn] = useState(initial.on);
  const [point, setPoint] = useState(initial.point);
  const [position, setPosition] = useState(initial.position);
  const [orientation, setOrientation] = useState(initial.orientation);
  const changed =
    mountOn !== initial.on ||
    point !== initial.point ||
    position !== initial.position ||
    orientation !== initial.orientation;
  const text: CommandText = {
    command: "part_place",
    part: part.id,
    on: mountOn,
    point,
    position,
    orientation,
  };
  // 欄を変えたら service で Op にしてみる（読めなければ理由を出し、適用させない）
  const made = useCommandOps(changed ? text : null);
  const place = () => {
    if (made.ops) apply.mutate({ ops: made.ops, summary: t.summary(part.id) });
  };
  const unit = m?.authored_position?.unit ?? "m";
  return (
    <PropertyList>
      <PropertyRow label={t.mountOn}>
        <span className="flex gap-1.5">
          <NativeSelect
            size="sm"
            aria-label={t.mountPart}
            value={mountOn}
            placeholder={t.body}
            onChange={(e) => setMountOn(e.target.value)}
            options={design.parts
              .filter((p) => p.id !== part.id)
              .map((p) => ({ value: p.id, label: p.id }))}
          />
          <TextInput
            aria-label={t.point}
            mono
            className="h-7 w-28"
            value={point}
            placeholder={t.point}
            onChange={(e) => setPoint(e.target.value)}
          />
        </span>
      </PropertyRow>
      <PropertyRow label={t.position}>
        <TextInput
          aria-label={t.position}
          mono
          className="h-7 w-52"
          value={position}
          placeholder={t.positionPlaceholder}
          onChange={(e) => setPosition(e.target.value)}
        />
      </PropertyRow>
      <PropertyRow label={t.orientation}>
        <TextInput
          aria-label={t.orientation}
          mono
          className="h-7 w-52"
          value={orientation}
          placeholder={QUATERNION_EXAMPLE}
          onChange={(e) => setOrientation(e.target.value)}
        />
      </PropertyRow>
      <PropertyRow label={t.bodyFrame}>
        <span className="font-mono text-[11.5px] text-ink-3">
          {m ? (
            <>
              (
              {m.body_position.map((v, i) => (
                <span key={["x", "y", "z"][i]}>
                  {i > 0 && ", "}
                  <Formatted si={v} dimension="Length" unit={unit} fallback={String(v)} />
                </span>
              ))}
              )
            </>
          ) : (
            "—"
          )}
        </span>
      </PropertyRow>
      <div className="flex items-center justify-end gap-3 px-3 py-1.5">
        {made.error && <span className="min-w-0 text-[11px] text-neg-ink">{made.error}</span>}
        <Button size="sm" variant="outline" disabled={!made.ops || apply.isPending} onClick={place}>
          {t.apply}
        </Button>
      </div>
    </PropertyList>
  );
}
