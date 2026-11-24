import { useQueries } from "@tanstack/react-query";
import { useState } from "react";
import { useApplyOps, useCommandOps } from "@/api/ops";
import { q } from "@/api/queries";
import type { CommandText, DesignView, PartRow, PoseQuantities } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/detail/PropertyList";
import { SkeletonBlock } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { Pill } from "@/kit/status/Pill";
import { idWithName, poseOverridden } from "@/lib/design";
import {
  AXES,
  numbersInputText,
  rotationText,
  vectorInputText,
  vectorText,
} from "@/quantity/format";
import { Button } from "@/ui/button";

/** 配置の欄に打った文字列（角度は X・Y・Z の 3 つの欄。度） */
export interface PlacementForm {
  on: string;
  point: string;
  position: string;
  angles: readonly [string, string, string];
}

type Placement = ReturnType<typeof useT>["components"]["placement"];

/** 欄の初期値: 位置は書かれたものだけを書かれた単位で、角度は API の取付点に対する姿勢の角度（度）で */
export function initialForm(part: PartRow): PlacementForm {
  const m = part.mount;
  // 取付の無い部品（根）の `[pose]` は機体座標での姿勢。書かれた値と角度は API の body から読む
  const r = m ? m.relative.rotation : part.body?.quantities.rotation;
  const authored = m ? m.authored_position : part.body?.authored_position;
  return {
    on: m?.on ?? "",
    point: m?.point ?? "",
    // 書かれていなければ空（空の欄は CLI で旗を書かないのと同じで、書かない）
    position: authored ? vectorInputText(authored) : "",
    angles: r ? [String(r[0].value), String(r[1].value), String(r[2].value)] : ["0", "0", "0"],
  };
}

function sameForm(a: PlacementForm, b: PlacementForm): boolean {
  return (
    a.on === b.on &&
    a.point === b.point &&
    a.position === b.position &&
    a.angles.every((v, i) => v === b.angles[i])
  );
}

/**
 * `fune part place` の引数。角度を変えたら `--rotation "x y z deg"`、変えなければ書かれた四元数をそのまま
 * `--orientation` で送る（読むのはどちらも service）。
 */
export function placeText(
  part: PartRow,
  form: PlacementForm,
  initial: PlacementForm,
  configuration?: string | null,
): CommandText {
  const turned = form.angles.some((v, i) => v !== initial.angles[i]);
  // 取付の無い部品（根）の姿勢は機体座標の `[pose]` の書かれた値
  const orientation = part.mount
    ? part.mount.authored_orientation
    : part.body?.authored_orientation;
  // 位置を空にした場合も、形態の姿勢には位置か姿勢のどちらかが必要。
  const writeAngles = turned || (configuration && !form.position.trim() && !orientation);
  return {
    command: "part_place",
    part: part.id,
    ...(configuration ? { configuration } : { on: form.on, point: form.point }),
    position: form.position,
    orientation:
      !writeAngles && orientation
        ? numbersInputText([orientation.w, orientation.x, orientation.y, orientation.z])
        : null,
    rotation: writeAngles ? `${form.angles.join(" ")} deg` : null,
  };
}

/**
 * 配置: 取付先と取付点（取付先の部品の `mount_points` から選ぶ）、取付点からの位置、姿勢（X・Y・Z 軸まわりの角度）。
 * 位置と角度は `fune part place --position "0 0 12 mm" --rotation "0 90 0 deg"` と同じ文字列にして
 * `POST /ops/from_command` に送る（読むのは service）。取付点・機体座標の位置と姿勢は API の表示のまま（位置は同じ単位）。
 */
export function PlacementSection({
  design,
  part,
  configuration,
}: {
  design: DesignView;
  part: PartRow;
  configuration?: string | null;
}) {
  const apply = useApplyOps();
  const t = useT().components.placement;
  const [initial] = useState(() => initialForm(part));
  const [form, setForm] = useState(initial);
  const made = useCommandOps(
    sameForm(form, initial) ? null : placeText(part, form, initial, configuration),
  );
  const edit = (patch: Partial<PlacementForm>) => setForm((f) => ({ ...f, ...patch }));
  const place = () => {
    if (made.ops) apply.mutate({ ops: made.ops, summary: t.summary(part.id) });
  };
  return (
    <PropertyList className="@container min-w-0">
      {configuration && poseOverridden(design, configuration, part) && (
        <div className="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
          <Pill tone="accent" dot={false}>
            {t.overridden}
          </Pill>
          <Button
            size="sm"
            variant="outline"
            disabled={apply.isPending}
            onClick={() =>
              apply.mutate({
                ops: [{ op: "unset_configuration_pose", configuration, part: part.id }],
                summary: t.resetSummary(part.id),
              })
            }
          >
            {t.resetOverride}
          </Button>
        </div>
      )}
      <MountFields
        design={design}
        part={part}
        form={form}
        onChange={edit}
        t={t}
        disabled={Boolean(configuration)}
      />
      <Readout label={t.pointPose} pose={pointPose(design, part, form)} empty={t.origin} />
      <Field
        className="px-3 py-2 wrap-anywhere"
        label={t.position}
        hint={form.position.trim() === "" ? t.positionUnset : undefined}
      >
        <TextInput
          aria-label={t.position}
          mono
          size="sm"
          value={form.position}
          placeholder={t.positionPlaceholder}
          onChange={(e) => edit({ position: e.target.value })}
        />
      </Field>
      <Field className="px-3 py-2 wrap-anywhere" label={t.orientation} hint={t.rotationHint}>
        <span className="grid min-w-0 grid-cols-3 gap-1.5">
          {AXES.map((axis, i) => (
            <span key={axis} className="flex min-w-0 items-center gap-1 text-[11.5px] text-ink-3">
              {axis}
              <TextInput
                aria-label={t.axisAngle(axis)}
                mono
                size="sm"
                value={form.angles[i]}
                onChange={(e) => {
                  const angles: [string, string, string] = [...form.angles];
                  angles[i] = e.target.value;
                  edit({ angles });
                }}
              />
              °
            </span>
          ))}
        </span>
      </Field>
      <Readout label={t.bodyFrame} pose={part.body?.quantities ?? null} empty="—" />
      <div className="flex flex-wrap items-center justify-end gap-3 px-3 py-1.5">
        {made.error && <span className="min-w-0 text-[11px] text-neg-ink">{made.error}</span>}
        <Button size="sm" variant="outline" disabled={!made.ops || apply.isPending} onClick={place}>
          {t.apply}
        </Button>
      </div>
    </PropertyList>
  );
}

/** 選んでいる取付点の位置・姿勢。今の取付点なら部品と同じ単位の `point_pose`、選び直した点は取付先の一覧のもの */
function pointPose(design: DesignView, part: PartRow, form: PlacementForm): PoseQuantities | null {
  if (form.point === "") return null;
  const m = part.mount;
  if (m && m.on === form.on && m.point === form.point) return m.point_pose;
  const parent = design.parts.find((p) => p.id === form.on);
  return parent?.mount_points.find((p) => p.name === form.point)?.pose ?? null;
}

/** 取付先の部品と、その部品の取付点（名前と位置）の選択肢。取付先を変えたら取付点は原点に戻す */
function MountFields({
  design,
  part,
  form,
  onChange,
  t,
  disabled,
}: {
  design: DesignView;
  part: PartRow;
  form: PlacementForm;
  onChange: (patch: Partial<PlacementForm>) => void;
  t: Placement;
  disabled: boolean;
}) {
  const points = design.parts.find((p) => p.id === form.on)?.mount_points ?? [];
  const options = points.map((p) => ({
    value: p.name,
    label: `${p.name} ${vectorText(p.pose.position)}`,
  }));
  // 書かれた取付点が取付先に無いときも、選んでいるものとして出す（検査が報告する）
  if (form.point !== "" && !points.some((p) => p.name === form.point)) {
    options.push({ value: form.point, label: t.missingPoint(form.point) });
  }
  return (
    <Field className="px-3 py-2 wrap-anywhere" label={t.mountOn}>
      <span className="grid min-w-0 grid-cols-1 gap-1.5 @min-[260px]:grid-cols-2">
        <NativeSelect
          size="sm"
          aria-label={t.mountPart}
          disabled={disabled}
          value={form.on}
          placeholder={t.body}
          onChange={(e) => onChange({ on: e.target.value, point: "" })}
          options={design.parts
            .filter((p) => p.id !== part.id)
            .map((p) => ({ value: p.id, label: p.id }))}
        />
        <NativeSelect
          size="sm"
          aria-label={t.point}
          value={form.point}
          placeholder={t.origin}
          disabled={disabled || form.on === ""}
          onChange={(e) => onChange({ point: e.target.value })}
          options={options}
        />
      </span>
    </Field>
  );
}

/** 導出した位置・姿勢の表示（API の display を並べるだけ） */
function Readout({
  label,
  pose,
  empty,
}: {
  label: string;
  pose: PoseQuantities | null;
  empty: string;
}) {
  return (
    <Field className="px-3 py-2 wrap-anywhere" label={label}>
      <span className="grid min-w-0 gap-0.5 font-mono text-[11.5px] text-ink-3 wrap-anywhere">
        {pose ? (
          <>
            <span className="num">{vectorText(pose.position)}</span>
            <span className="num">{rotationText(pose.rotation)}</span>
          </>
        ) : (
          empty
        )}
      </span>
    </Field>
  );
}

/** 部品詳細では上書きがある形態だけを読み、取付点に対する姿勢を表示する。 */
export function PlacementOverrides({ design, part }: { design: DesignView; part: PartRow }) {
  const t = useT().components.placement;
  const overrides = design.configurations.filter(({ overridden }) => overridden.includes(part.id));
  const queries = useQueries({
    queries: overrides.map(({ id }) => q.part(part.id, [], id)),
  });
  if (overrides.length === 0) return null;
  return (
    <div className="mt-3 space-y-1">
      <h3 className="font-medium text-ink-2 text-xs">{t.overrides}</h3>
      <PropertyList>
        {overrides.map((configuration, index) => (
          <QueryState
            key={configuration.id}
            query={queries[index]!}
            skeleton={<SkeletonBlock className="h-9" />}
          >
            {({ part }) => (
              <Readout
                label={idWithName(configuration)}
                pose={part.mount?.overridden ? part.mount.relative : null}
                empty="—"
              />
            )}
          </QueryState>
        ))}
      </PropertyList>
    </div>
  );
}
