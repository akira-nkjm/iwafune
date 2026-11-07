import { useState } from "react";
import { useApplyOps } from "@/api/ops";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/NativeSelect";
import { PropertyList } from "@/kit/PropertyList";
import { PropertyRow } from "@/kit/PropertyRow";
import { TextInput } from "@/kit/TextInput";
import { Formatted } from "@/quantity/Formatted";
import { Button } from "@/ui/button";

type Vec = [string, string, string];
type Q = [string, string, string, string];

const str = (a: readonly number[] | null | undefined, n: number, fill: number[]) =>
  (a ?? fill).slice(0, n).map((v) => String(v));

/**
 * 配置: 取付先と取付点、取付点に対する位置・姿勢（Place）。位置はファイルに書かれた単位（`authored_position`）で
 * 見せてその単位で送る（CLI で同じ単位を打つのと同じファイルになる。換算はしない）。書かれていなければ m。
 * 機体座標は取付をたどった導出値で、表示だけ。
 */
export function PlacementSection({ design, part }: { design: DesignView; part: PartRow }) {
  const apply = useApplyOps();
  const t = useT().components.placement;
  const m = part.mount;
  const authored = m?.authored_position;
  const unit = authored?.unit ?? "m";
  const [mountOn, setMountOn] = useState(m?.on ?? "");
  const [point, setPoint] = useState(m?.point ?? "");
  const [pos, setPos] = useState<Vec>(
    (authored
      ? [authored.x, authored.y, authored.z].map(String)
      : str(m?.position, 3, [0, 0, 0])) as Vec,
  );
  const [rot, setRot] = useState<Q>(str(m?.orientation, 4, [1, 0, 0, 0]) as Q);
  const nums = [...pos, ...rot].map(Number);
  const ok = nums.every(Number.isFinite);
  const place = () => {
    if (!ok) return;
    const [x, y, z, w, qx, qy, qz] = nums as [
      number,
      number,
      number,
      number,
      number,
      number,
      number,
    ];
    apply.mutate({
      ops: [
        {
          op: "place",
          part: part.id,
          on: mountOn || null,
          point: mountOn ? point || null : null,
          position: { x, y, z, unit },
          orientation: { w, x: qx, y: qy, z: qz },
        },
      ],
      summary: t.summary(part.id),
    });
  };
  const body = m?.body_position;
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
        <span className="flex items-center gap-1">
          {(["x", "y", "z"] as const).map((k, i) => (
            <TextInput
              key={k}
              aria-label={t.positionAxis(k)}
              mono
              className="h-7 w-20 text-right"
              value={pos[i]}
              onChange={(e) => setPos(pos.map((v, j) => (j === i ? e.target.value : v)) as Vec)}
            />
          ))}
          <span className="font-mono text-[11.5px] text-ink-3">{unit}</span>
        </span>
      </PropertyRow>
      <PropertyRow label={t.orientation}>
        <span className="flex items-center gap-1">
          {(["w", "x", "y", "z"] as const).map((k, i) => (
            <TextInput
              key={k}
              aria-label={t.orientationAxis(k)}
              mono
              className="h-7 w-16 text-right"
              value={rot[i]}
              onChange={(e) => setRot(rot.map((v, j) => (j === i ? e.target.value : v)) as Q)}
            />
          ))}
        </span>
      </PropertyRow>
      <PropertyRow label={t.bodyFrame}>
        <span className="font-mono text-[11.5px] text-ink-3">
          {body ? (
            <>
              (
              {body.map((v, i) => (
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
      <div className="flex justify-end px-3 py-1.5">
        <Button size="sm" variant="outline" disabled={!ok || apply.isPending} onClick={place}>
          {t.apply}
        </Button>
      </div>
    </PropertyList>
  );
}
