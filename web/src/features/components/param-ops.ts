import { setValueOp } from "@/api/op-values";
import type { Op, Provenance, QuantityView } from "@/api/types";
import type { NewAttribute } from "./NewAttributeFields";

/** 「パラメーター追加」の Op: 標準の属性なら SetValue だけ、新しい属性なら CreateAttribute + SetValue */
export function paramOps(args: {
  part: string;
  mode: "standard" | "new";
  attrId: string;
  hasDef: boolean;
  fresh: NewAttribute;
  state: string | null;
  value: QuantityView | null;
  from: Provenance | null;
}): Op[] | null {
  const { part, mode, attrId, fresh, state, value, from } = args;
  if (!attrId || !value) return null;
  const set: Op = { ...setValueOp(part, attrId, state, value), from };
  if (mode === "standard") return args.hasDef ? [set] : null;
  if (!fresh.dimension) return null;
  return [
    {
      op: "create_attribute",
      def: {
        id: fresh.id,
        label: fresh.label || fresh.id,
        type: "scalar",
        dimension: fresh.dimension,
        units: fresh.units,
        by_state: fresh.byState,
      },
    },
    set,
  ];
}
