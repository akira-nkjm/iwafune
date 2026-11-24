import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { q } from "@/api/queries";
import type { DesignView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { SkeletonTable } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { DataTable } from "@/kit/table/DataTable";
import { TablePanel } from "@/kit/table/TablePanel";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { QuantityInput } from "@/quantity/QuantityInput";
import { useDimensionLabel } from "@/quantity/use-dimension-label";

/** 単位の一覧（sat-units のカーネル）と、その次元を使っている属性。入力の試し打ちもここでできる */
export function UnitsTab({ design }: { design: DesignView | undefined }) {
  const t = useT();
  const dimensionLabel = useDimensionLabel();
  const tu = t.defs.units;
  const mappingLabel = {
    linear: tu.linear,
    affine: tu.affine,
    logarithmic: tu.logarithmic,
  } as const;
  const [dim, setDim] = useState("");
  const all = useQuery(q.units(null));
  const filtered = useQuery({ ...q.units(dim), enabled: dim !== "" });
  const data = dim ? filtered.data : all.data;
  const usedBy = (d: string) =>
    (design?.defs.attributes ?? []).filter((a) => a.dimension === d).map((a) => a.id);
  return (
    <>
      <Panel>
        <PanelHeader title={tu.tryTitle} caption={tu.tryCaption} />
        <QuantityInput
          initial="18 mN*m*s"
          dimension={dim || null}
          label={tu.tryLabel}
          className="max-w-md"
        />
      </Panel>
      <TablePanel
        title={tu.title}
        caption={data ? tu.caption(data.units.length, data.dimensions.length) : undefined}
        right={
          <NativeSelect
            aria-label={tu.filter}
            value={dim}
            onChange={(e) => setDim(e.target.value)}
            placeholder={tu.allDimensions}
            options={(all.data?.dimensions ?? []).map((d) => ({
              value: d.name,
              label: localized(d.label, d.label_en),
            }))}
          />
        }
      >
        <QueryState
          query={dim ? filtered : all}
          failure={tu.cannotLoad}
          skeleton={<SkeletonTable rows={10} cols={5} />}
        >
          {(data) => (
            <DataTable
              head={
                <>
                  <Th>{tu.symbol}</Th>
                  <Th>{tu.dimension}</Th>
                  <Th>{tu.mapping}</Th>
                  <Th align="right">{tu.scale}</Th>
                  <Th>{tu.usedBy}</Th>
                </>
              }
            >
              {data.units.map((u) => (
                <tr key={u.symbol}>
                  <Td mono>{u.symbol}</Td>
                  <Td>{dimensionLabel(u.dimension)}</Td>
                  <Td className="text-ink-3">{mappingLabel[u.mapping]}</Td>
                  <Td align="right" mono>
                    {u.scale}
                    {u.offset !== 0 ? ` + ${u.offset}` : ""}
                  </Td>
                  <Td mono className="text-ink-3">
                    {usedBy(u.dimension).join(", ")}
                  </Td>
                </tr>
              ))}
            </DataTable>
          )}
        </QueryState>
      </TablePanel>
    </>
  );
}
