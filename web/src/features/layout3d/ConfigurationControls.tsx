import { useId, useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { ConfigurationText, ConfigurationView } from "@/api/types";
import { useT } from "@/i18n";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { idWithName } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import { Button } from "@/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

/** 配置の表示・編集先を選び、同じ場所から形態のレコードを編集する。 */
export function ConfigurationControls({
  configurations,
  selected,
  onSelect,
}: {
  configurations: readonly ConfigurationView[];
  selected: ConfigurationView | null;
  onSelect: (id: string | null, options?: { replace: boolean }) => void;
}) {
  const t = useT();
  const c = t.systems.layout3d.configuration;
  const options = configurations.map((c) => ({
    // 表示の言語の名前（英語は ID、日本語は「ID（表示名）」。`idWithName`）
    value: c.id,
    label: idWithName(c),
  }));
  const dialogs = useDialogs<"add" | "edit" | "rename" | "remove">();
  const [target, setTarget] = useState<ConfigurationView | null>(null);
  const open = (action: "edit" | "rename" | "remove") => {
    setTarget(selected);
    dialogs.show(action);
  };
  return (
    <div className="mb-2 flex flex-wrap items-center gap-2">
      <NativeSelect
        size="sm"
        aria-label={c.select}
        value={selected?.id ?? ""}
        placeholder={c.base}
        options={options}
        onChange={(e) => onSelect(e.target.value || null)}
      />
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button size="sm" variant="outline">
            {c.manage}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onSelect={() => dialogs.show("add")}>{c.add}</DropdownMenuItem>
          <DropdownMenuItem disabled={!selected} onSelect={() => open("edit")}>
            {c.edit}
          </DropdownMenuItem>
          <DropdownMenuItem disabled={!selected} onSelect={() => open("rename")}>
            {c.rename}
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={!selected}
            variant="destructive"
            onSelect={() => open("remove")}
          >
            {c.remove}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <p className="rounded-control bg-accent-tint px-2 py-1 text-accent-ink text-xs">
        {c.editing(options.find(({ value }) => value === selected?.id)?.label ?? c.base)}
      </p>
      {dialogs.current === "add" && <ConfigurationDialog {...dialogs.props} />}
      {dialogs.current === "edit" && target && (
        <ConfigurationDialog {...dialogs.props} initial={target.text} />
      )}
      {dialogs.current === "rename" && target && (
        <RenameDialog
          {...dialogs.props}
          from={target.id}
          title={c.renameTitle(target.id)}
          label={c.newId}
          opsFor={(to) => [{ op: "rename_configuration", id: target.id, to }]}
          summary={(to) => c.renamed(target.id, to)}
          onDone={(id) => onSelect(id, { replace: true })}
        />
      )}
      {dialogs.current === "remove" && target && (
        <OpDialog
          {...dialogs.props}
          title={c.removeTitle(target.id)}
          description={c.removeDescription}
          ops={[{ op: "remove_configuration", id: target.id }]}
          summary={c.removeTitle(target.id)}
          confirmLabel={c.remove}
          // 消す前に基準へ移る（消した後の再読込が、消した形態の部品一覧を読みに行かないように）
          onBeforeApply={() => onSelect(null, { replace: true })}
          destructive
        />
      )}
    </div>
  );
}

/** フェーズの編集と同様に、入力をサービスで Op に変換し、共通ダイアログで確認する。 */
function ConfigurationDialog({
  initial,
  open,
  onOpenChange,
}: {
  initial?: ConfigurationText;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const c = useT().systems.layout3d.configuration;
  const id = useId();
  const [text, setText] = useState<ConfigurationText>(initial ?? { id: "" });
  const made = useCommandOps(
    text.id.trim()
      ? { ...text, command: initial ? "configuration_set" : "configuration_add" }
      : null,
  );
  const input = (key: keyof ConfigurationText, label: string, disabled = false) => (
    <Field label={label} htmlFor={`${id}-${key}`}>
      <TextInput
        id={`${id}-${key}`}
        mono={key === "id"}
        value={text[key] ?? ""}
        disabled={disabled}
        onChange={(e) => setText({ ...text, [key]: e.target.value })}
      />
    </Field>
  );
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={initial ? c.editTitle(initial.id) : c.add}
      ops={made.ops}
      summary={initial ? c.editTitle(initial.id) : c.added(text.id.trim())}
      confirmLabel={initial ? undefined : c.add}
    >
      {input("id", c.id, Boolean(initial))}
      {input("label", c.label)}
      {input("description", c.description)}
      {made.error && (
        <p role="alert" className="text-neg-ink text-xs">
          {made.error}
        </p>
      )}
    </OpDialog>
  );
}
