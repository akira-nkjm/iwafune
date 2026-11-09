import type { ReactNode } from "react";
import type { AttributeDefView, PartRow, QuantityView, ValueView } from "@/api/types";
import { useT } from "@/i18n";
import { type PropertyLayout, PropertyRow } from "@/kit/PropertyRow";
import { paramPath } from "@/lib/design";
import { Popover, PopoverContent, PopoverTrigger } from "@/ui/popover";
import { attrEditMode, cellValue } from "./attr-mode";
import { EditableQuantity } from "./EditableQuantity";
import { quantityInputText, scalarOf, valueText } from "./format";

/** 値を持つもの（部品・カタログ品）。パスは API の `paths`、状態は種別の語彙 */
type Owner = Pick<PartRow, "paths" | "states">;

/** 状態の並び: 持ち主の状態の語彙。無ければ値に書かれた状態 */
function statesOf(owner: Owner, v: ValueView | undefined): string[] {
  if (owner.states.length > 0) return owner.states;
  return v?.value.type === "by_state" ? Object.keys(v.value.states) : [];
}

interface Common {
  owner: Owner;
  attr: string;
  def: AttributeDefView | undefined;
  value: ValueView | undefined;
  /** 確定: 状態（属性全体なら null）と打った文字列（型は service が決める。`useAttrEdit` の `set` に渡す） */
  onSet: (state: string | null, text: string) => void;
  disabled?: boolean;
}

/** 詳細の行（部品の詳細・カタログ品の詳細）。状態ごとの値は状態ごとの子の行に分ける */
interface RowProps extends Common {
  layout: "row";
  rowLayout: Extract<PropertyLayout, "values" | "action">;
  label: ReactNode;
  /** 行の右の出所（rowLayout が values のとき） */
  source?: ReactNode;
  /** 行の右端の操作（戻す・消す） */
  action?: ReactNode;
}

/** 表のセル（系のページ）。`state` を渡すとその状態の 1 セル、無ければ状態ごとの値は開いて直す */
interface CellProps extends Common {
  layout: "cell";
  state?: string | null;
}

/**
 * 属性の値の表示と編集（部品とカタログ品で共通）。スカラー・状態ごと・表示だけの分岐はここだけで行い、
 * 打った文字列はそのまま `onSet` に渡す（呼び手は `api/ops.ts` の `useAttrEdit`）。
 */
export function AttrValueEditor(props: RowProps | CellProps) {
  return props.layout === "row" ? <AttrRowEditor {...props} /> : <AttrCellEditor {...props} />;
}

/** 量 1 つの欄。label はパラメーターのパス（検証で弾かれた理由をその欄の下に出す） */
function QuantityField({
  owner,
  attr,
  def,
  state,
  q,
  display,
  muted,
  disabled,
  onSet,
}: Common & {
  state: string | null;
  q: QuantityView | null;
  display: string;
  muted: boolean;
}) {
  return (
    <EditableQuantity
      display={display}
      initial={quantityInputText(q)}
      dimension={def?.dimension ?? q?.dimension ?? null}
      label={paramPath(owner, attr, state)}
      muted={muted}
      disabled={disabled}
      onCommit={(_q, text) => onSet(state, text)}
    />
  );
}

function DisplayOnly({ value }: { value: ValueView | undefined }) {
  return (
    <span className="block truncate font-mono text-[12px]" title={valueText(value)}>
      {valueText(value)}
    </span>
  );
}

const unset = (v: ValueView | undefined) => !v || v.origin === "unset" || v.value.type === "unset";

/** 属性全体の欄（スカラー、または状態ごとの属性にどの状態でも同じ値が書かれているとき） */
function WholeField(props: Common) {
  const { value } = props;
  return (
    <QuantityField
      {...props}
      state={null}
      q={scalarOf(value)}
      display={valueText(value)}
      muted={unset(value)}
    />
  );
}

/** 状態 1 つの欄 */
function StateField(props: Common & { state: string }) {
  const q = cellValue(props.value, props.state);
  return <QuantityField {...props} q={q} display={q ? q.display : "—"} muted={!q} />;
}

function AttrRowEditor(props: RowProps) {
  const t = useT();
  const { owner, attr, def, value, rowLayout, label, source, action } = props;
  const mode = attrEditMode(def);
  const head = (children: ReactNode) => (
    <PropertyRow layout={rowLayout} label={label} title={attr} source={source} action={action}>
      {children}
    </PropertyRow>
  );
  if (mode === "display") return head(<DisplayOnly value={value} />);
  if (mode === "scalar") return head(<WholeField {...props} />);
  return (
    <div className="py-1">
      {head(
        value?.value.type === "scalar" ? (
          <WholeField {...props} />
        ) : (
          <span className="text-[11px] text-ink-4">{t.quantity.byState}</span>
        ),
      )}
      {statesOf(owner, value).map((s) => (
        <PropertyRow key={s} layout={rowLayout} label={s} indent className="py-0.5">
          <StateField {...props} state={s} />
        </PropertyRow>
      ))}
    </div>
  );
}

function AttrCellEditor(props: CellProps) {
  const { owner, attr, value, state } = props;
  const mode = attrEditMode(props.def);
  if (mode === "display" && state)
    return <span className="font-mono text-[12px]">{cellValue(value, state)?.display ?? "—"}</span>;
  if (mode === "display") return <DisplayOnly value={value} />;
  if (mode === "scalar") return <WholeField {...props} />;
  if (state) return <StateField {...props} state={state} />;
  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          className="max-w-56 truncate rounded-[6px] px-1 py-0.5 text-left hover:bg-accent-tint"
          title={valueText(value)}
        >
          {valueText(value)}
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-2">
        <p className="mb-1 px-1 font-mono text-[11px] text-ink-3">{paramPath(owner, attr)}</p>
        {statesOf(owner, value).map((s) => (
          <div key={s} className="grid grid-cols-[70px_1fr] items-center gap-2 px-1 py-0.5 text-xs">
            <span className="font-mono text-ink-3">{s}</span>
            <StateField {...props} state={s} />
          </div>
        ))}
      </PopoverContent>
    </Popover>
  );
}
