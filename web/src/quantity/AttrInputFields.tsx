import type { ReactNode } from "react";
import type { AttrInput, ValueView } from "@/api/types";
import { useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";
import { choiceLabel } from "@/kit/forms/choice-label";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { useFieldError } from "@/lib/field-errors";
import { choiceText } from "./attr-mode";
import { valueText } from "./format";

/** 量以外の入力欄（選ぶ・部品を選ぶ・打つ・直せない）に渡すもの。`label` はパラメーターのパス */
export interface InputFieldProps<K extends AttrInput["kind"]> {
  input: Extract<AttrInput, { kind: K }>;
  value: ValueView | undefined;
  label: string;
  /** 確定: 選んだ・打った文字列（型は service が決める。`useAttrEdit` の `set` に渡す） */
  onCommit: (text: string) => void;
  disabled?: boolean;
}

/** 検証で弾かれた理由をその欄（パラメーターのパス）の下に出す */
function WithError({ label, children }: { label: string; children: ReactNode }) {
  const error = useFieldError(label);
  if (!error) return <>{children}</>;
  return (
    <span className="inline-flex min-w-0 flex-col">
      {children}
      <span className="px-1 text-[11px] text-neg-ink">{error}</span>
    </span>
  );
}

/** 選び直した値（今と同じ・空の選択肢（未設定）なら null。未設定に戻すのは「戻す」の操作） */
export function changedChoice(current: string, picked: string): string | null {
  return picked && picked !== current ? picked : null;
}

/** 候補（API が返した選べる値）から選ぶ欄。今の値が候補に無くても（語彙の外・消えた部品）見失わないよう先頭に足す */
function SelectField({
  choices,
  hint,
  value,
  label,
  onCommit,
  disabled,
}: Pick<InputFieldProps<"choice">, "value" | "label" | "onCommit" | "disabled"> & {
  choices: Extract<AttrInput, { kind: "choice" }>["choices"];
  /** 候補が無いときに出す案内（足す CLI のコマンド） */
  hint?: string | null;
}) {
  const t = useT().quantity;
  const current = choiceText(value);
  const options = choices.map((c) => ({ value: c.value, label: choiceLabel(c) }));
  if (current && !options.some((o) => o.value === current))
    options.unshift({ value: current, label: current });
  const none = options.length === 0;
  return (
    <WithError label={label}>
      <NativeSelect
        size="sm"
        className="w-48 max-w-full font-mono"
        aria-label={t.edit(label)}
        title={none ? (hint ?? undefined) : label}
        disabled={disabled || none}
        value={current}
        placeholder={none ? t.noCandidates : current ? undefined : t.unsetChoice}
        options={options}
        onChange={(e) => {
          const next = changedChoice(current, e.target.value);
          if (next !== null) onCommit(next);
        }}
      />
    </WithError>
  );
}

/** 決まった語彙（列挙・真偽）から選ぶ */
function ChoiceField({ input, ...rest }: InputFieldProps<"choice">) {
  return <SelectField choices={input.choices} {...rest} />;
}

/** 設計にあるもの（部品）から選ぶ。候補が無ければ足す CLI のコマンドを案内する */
function ReferenceField({ input, ...rest }: InputFieldProps<"reference">) {
  return <SelectField choices={input.choices} hint={input.add_command} {...rest} />;
}

/** 自由に打つ（Enter か欄を離れたときに確定。空では確定しない） */
function TextField({ value, label, onCommit, disabled }: InputFieldProps<"text">) {
  return (
    <WithError label={label}>
      <CommitInput
        mono
        allowEmpty={false}
        className="w-48 max-w-full"
        label={label}
        value={choiceText(value)}
        onCommit={onCommit}
        disabled={disabled}
      />
    </WithError>
  );
}

/** 欄では直せない値（理由は値の型。ホバーで直し方を出す） */
function ReadOnlyField({ input, value }: InputFieldProps<"read_only">) {
  const t = useT().quantity;
  return (
    <span
      className="block truncate font-mono text-[12px]"
      title={`${valueText(value)}\n${t.readOnly[input.reason]}`}
    >
      {valueText(value)}
    </span>
  );
}

/** 量以外の入力の種類 → 欄の部品（種類を足すと、ここに足さない限り型エラーになる） */
export const INPUT_FIELDS: {
  [K in Exclude<AttrInput["kind"], "quantity">]: (p: InputFieldProps<K>) => ReactNode;
} = {
  choice: ChoiceField,
  reference: ReferenceField,
  text: TextField,
  read_only: ReadOnlyField,
};

/** 量以外の入力の種類 */
type OtherInput = Exclude<AttrInput, { kind: "quantity" }>;

/** 量以外の入力の欄（種類は `INPUT_FIELDS` の表で欄の部品に写す） */
export function InputField({
  input,
  ...rest
}: { input: OtherInput } & Omit<InputFieldProps<"text">, "input">) {
  // 表の関数の引数は種類ごとに違う（`input` だけが違う）ので、種類の和に合わせて呼ぶ
  const render = INPUT_FIELDS[input.kind] as (p: { input: OtherInput } & typeof rest) => ReactNode;
  return <>{render({ input, ...rest })}</>;
}
