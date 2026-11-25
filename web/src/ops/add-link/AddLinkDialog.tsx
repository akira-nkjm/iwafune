import { useState } from "react";
import { useEnumOptions } from "@/api/labels";
import { useCommandOps } from "@/api/ops";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { bandOptions, emptyLink, linkAddCommand, transceiverOptions } from "./link-form";

/**
 * 通信の回線の追加（CreateCommLink。`fune link add` と同じ引数）。通信のページのボタンと、足りない入力の行（`AddListItem`）から開く。
 * 向き・送受信機・帯域は API が返した選択肢（語彙の値・`add_choices`）から選ぶ。`initialId` は足りない入力が名指しした回線の ID
 */
export function AddLinkDialog({
  design,
  initialId,
  onOpenChange,
}: {
  design: DesignView;
  initialId?: string | null;
  onOpenChange: (open: boolean) => void;
}) {
  const tl = useT().conditions.links;
  const [link, setLink] = useState(emptyLink(initialId));
  const directions = useEnumOptions("link_direction");
  const made = useCommandOps(linkAddCommand(link));
  const failed = useOpCheck(made.ops).failed;
  const error = made.error;
  const set = (key: keyof typeof link) => (e: { target: { value: string } }) =>
    setLink((l) => ({ ...l, [key]: e.target.value }));
  const parts = transceiverOptions(design);
  const bands = bandOptions(design.add_choices);
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={tl.addTitle}
      description={tl.addDescription}
      ops={made.ops}
      summary={tl.addSummary(link.id.trim())}
      confirmLabel={tl.addConfirm}
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label="ID" htmlFor="link-id" error={error}>
          <TextInput
            id="link-id"
            mono
            value={link.id}
            invalid={failed || !!error}
            placeholder="s_down"
            onChange={set("id")}
          />
        </Field>
        <Field label={tl.direction} htmlFor="link-direction">
          <NativeSelect
            id="link-direction"
            value={link.direction}
            placeholder={tl.choose}
            options={directions}
            onChange={set("direction")}
          />
        </Field>
        <Field
          label={tl.part}
          htmlFor="link-part"
          hint={parts.length === 0 ? tl.noPart : undefined}
        >
          <NativeSelect
            id="link-part"
            value={link.part}
            placeholder={tl.choose}
            options={parts}
            onChange={set("part")}
          />
        </Field>
        <Field
          label={tl.band}
          htmlFor="link-band"
          hint={bands.length === 0 ? tl.noBand : tl.bandHint}
        >
          <NativeSelect
            id="link-band"
            value={link.band}
            placeholder={tl.choose}
            options={bands}
            onChange={set("band")}
          />
        </Field>
      </div>
    </OpDialog>
  );
}
