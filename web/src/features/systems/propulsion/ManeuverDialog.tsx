import { useState } from "react";
import { useEnumOptions } from "@/api/labels";
import { useCommandOps } from "@/api/ops";
import type { DesignView, ManeuverText } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { maneuverCommand, thrusterOptions } from "./maneuver-form";

type TextKey = Exclude<keyof ManeuverText, "id" | "category" | "method">;

/**
 * マヌーバの追加・編集（`fune maneuver add|set` と同じ引数）。打った文字列のまま service に送り、Op にしてから確定する。
 * 編集は欄の初期値に API の `ManeuverView.text` を使い、丸ごと書き換える（`SetManeuver`）
 */
export function ManeuverDialog({
  design,
  initial,
  adding,
  onOpenChange,
}: {
  design: DesignView;
  initial: ManeuverText;
  adding: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().maneuvers;
  const td = t.dialog;
  const [text, setText] = useState<ManeuverText>(initial);
  const categories = useEnumOptions("maneuver_category");
  const methods = useEnumOptions("maneuver_method");
  const bodies = useEnumOptions("central_body");
  const steerings = useEnumOptions("maneuver_steering");
  const made = useCommandOps(maneuverCommand(text, adding));
  const failed = useOpCheck(made.ops).failed;
  const error = made.error;
  const set = (key: keyof ManeuverText, value: string) => setText((s) => ({ ...s, [key]: value }));
  const others = design.maneuvers.filter((m) => m.id !== text.id);
  const input = (key: TextKey, label: string, placeholder?: string) => (
    <Field label={label} htmlFor={`mv-${key}`}>
      <TextInput
        id={`mv-${key}`}
        mono
        value={text[key] ?? ""}
        placeholder={placeholder}
        onChange={(e) => set(key, e.target.value)}
      />
    </Field>
  );
  const select = (key: TextKey, label: string, options: { value: string; label: string }[]) => (
    <Field label={label} htmlFor={`mv-${key}`}>
      <NativeSelect
        id={`mv-${key}`}
        value={text[key] ?? ""}
        placeholder={td.unset}
        options={options}
        onChange={(e) => set(key, e.target.value)}
      />
    </Field>
  );
  return (
    <OpDialog
      open
      wide
      onOpenChange={onOpenChange}
      title={adding ? td.addTitle : td.editTitle(text.id)}
      description={td.description}
      ops={made.ops}
      summary={adding ? td.summaryAdd(text.id) : td.summaryEdit(text.id)}
      confirmLabel={adding ? td.confirmAdd : td.confirmEdit}
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label={t.id} htmlFor="mv-id" error={error}>
          <TextInput
            id="mv-id"
            mono
            value={text.id}
            disabled={!adding}
            invalid={failed || !!error}
            placeholder="RAISE"
            onChange={(e) => set("id", e.target.value)}
          />
        </Field>
        {input("label", td.label)}
        <Field label={t.category} htmlFor="mv-category">
          <NativeSelect
            id="mv-category"
            value={text.category}
            options={categories}
            onChange={(e) => set("category", e.target.value)}
          />
        </Field>
        <Field label={t.method} htmlFor="mv-method">
          <NativeSelect
            id="mv-method"
            value={text.method}
            options={methods}
            onChange={(e) => set("method", e.target.value)}
          />
        </Field>
        {select("thruster", t.thruster, thrusterOptions(design))}
        {select(
          "after",
          t.after,
          others.map((m) => ({ value: m.id, label: m.id })),
        )}
        {select("body", td.body, bodies)}
        {input("altitude", td.altitude, "600 km")}
        {input("inclination", td.inclination, "97.8 deg")}
        {input("perigee_altitude", td.perigeeAltitude, "250 km")}
        {input("apogee_altitude", td.apogeeAltitude)}
        {input("via_altitude", td.viaAltitude)}
        {select("sunlit_only", td.sunlitOnly, [
          { value: "true", label: td.yes },
          { value: "false", label: td.no },
        ])}
        {select(
          "mode",
          td.mode,
          design.modes.modes.map((m) => ({ value: m.id, label: m.id })),
        )}
        {input("duty", td.duty, "0.5")}
        {select("steering", td.steering, steerings)}
      </div>
    </OpDialog>
  );
}
