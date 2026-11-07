import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { q } from "@/api/queries";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonTable } from "@/kit/Loading";
import { NativeSelect } from "@/kit/NativeSelect";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { QuantityInput } from "@/quantity/QuantityInput";

/** 単位の一覧（sat-units のカーネル）と、その次元を使っている属性。入力の試し打ちもここでできる */
export function UnitsTab({ design }: { design: DesignView | undefined }) {
  const t = useT();
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
      <Panel pad={false}>
        <div className="flex items-center justify-between gap-2 p-4 pb-2">
          <PanelHeader
            className="mb-0"
            title={tu.title}
            caption={data ? tu.caption(data.units.length, data.dimensions.length) : undefined}
          />
          <NativeSelect
            aria-label={tu.filter}
            value={dim}
            onChange={(e) => setDim(e.target.value)}
            placeholder={tu.allDimensions}
            options={(all.data?.dimensions ?? []).map((d) => ({ value: d.name, label: d.name }))}
          />
        </div>
        {!data ? (
          all.isError ? (
            <Empty>{tu.cannotLoad(all.error.message)}</Empty>
          ) : (
            <Loading>
              <SkeletonTable rows={10} cols={5} />
            </Loading>
          )
        ) : (
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
                <Td>{u.dimension}</Td>
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
      </Panel>
    </>
  );
}
