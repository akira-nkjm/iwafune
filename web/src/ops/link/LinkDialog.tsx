import { useState } from "react";
import { useEnumOptions } from "@/api/labels";
import { useCommandOps, useTextOp } from "@/api/ops";
import type { DesignView, LinkView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import {
  bandOptions,
  emptyLink,
  type LINK_FIELDS,
  linkAddCommand,
  linkEditOps,
  linkFields,
  linkFieldText,
  transceiverOptions,
  withCurrent,
} from "./link-form";

/**
 * 通信の回線の追加と編集の共通のダイアログ。`editing` が無ければ追加（CreateCommLink。`fune link add` と同じ引数。通信のページの
 * ボタンと、足りない入力の行（`AddListItem`）から開く）、あれば開いた回線の編集（直した欄ごとに `fune set <回線の設定>.<欄>` と同じ
 * `POST /ops/from_text` で書き、ID を変えたら RenameCommLink）。向き・送受信機・帯域は API が返した選択肢（語彙の値・`add_choices`）から選ぶ。
 * `initialId` は足りない入力が名指しした回線の ID
 */
export function LinkDialog({
  design,
  editing,
  initialId,
  onOpenChange,
}: {
  design: DesignView;
  editing?: LinkView;
  initialId?: string | null;
  onOpenChange: (open: boolean) => void;
}) {
  const tl = useT().conditions.links;
  const [link, setLink] = useState(editing ? linkFields(editing) : emptyLink(initialId));
  const directions = useEnumOptions("link_direction");
  const added = useCommandOps(editing ? null : linkAddCommand(link));
  // 直した欄ごとに Op にする（`LINK_FIELDS` の順）
  const text = (field: (typeof LINK_FIELDS)[number]) =>
    editing ? linkFieldText(editing, link, field) : null;
  const edited = [useTextOp(text("direction")), useTextOp(text("part")), useTextOp(text("band"))];
  const fieldOps = edited.map((m) => m.op);
  const ops = editing ? linkEditOps(editing, link, fieldOps) : added.ops;
  const failed = useOpCheck(ops).failed;
  const error = editing ? (edited.find((m) => m.error)?.error ?? null) : added.error;
  const set = (key: keyof typeof link) => (e: { target: { value: string } }) =>
    setLink((l) => ({ ...l, [key]: e.target.value }));
  const parts = withCurrent(transceiverOptions(design), editing?.part ?? null);
  const bands = withCurrent(bandOptions(design.add_choices), editing?.band ?? null);
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={editing ? tl.editTitle(editing.id) : tl.addTitle}
      description={editing ? tl.editDescription : tl.addDescription}
      ops={ops}
      summary={editing ? tl.editSummary(editing.id) : tl.addSummary(link.id.trim())}
      confirmLabel={editing ? tl.editConfirm : tl.addConfirm}
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
