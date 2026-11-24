import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { ComplianceRow, ComplianceSummary } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonTable } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Refreshable } from "@/kit/feedback/Refreshable";
import { StatStrip } from "@/kit/metrics/StatStrip";
import { Panel } from "@/kit/panel/Panel";
import { Pill } from "@/kit/status/Pill";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { VerifiedBy } from "./VerifiedBy";

/**
 * 要求適合表: 要求ごとの判定（`GET /requirements/compliance`）。判定と件数の数え方は service が決め、ここは並べるだけ。
 * 設計を編集すると読み直す（前の表を薄くして出したまま）。
 */
export function ComplianceTab() {
  const t = useT().requirements;
  const report = useQuery(q.compliance());
  return (
    <QueryState query={report} skeleton={<SkeletonTable rows={6} cols={7} />}>
      {(r) =>
        r.rows.length === 0 ? (
          <Empty>{t.complianceEmpty}</Empty>
        ) : (
          <Refreshable active={report.isFetching}>
            <div className="flex flex-col gap-4">
              <Summary summary={r.summary} />
              <Panel pad={false}>
                <DataTable
                  head={
                    <>
                      <Th>{t.verdict}</Th>
                      <Th>{t.statement}</Th>
                      <Th>{t.constraint}</Th>
                      <Th>{t.verify}</Th>
                      <Th>{t.result}</Th>
                    </>
                  }
                >
                  {r.rows.map((row) => (
                    <ComplianceRowView key={row.id} row={row} />
                  ))}
                </DataTable>
              </Panel>
            </div>
          </Refreshable>
        )
      }
    </QueryState>
  );
}

function Summary({ summary }: { summary: ComplianceSummary }) {
  const t = useT().requirements.summary;
  return (
    <StatStrip
      items={[
        { key: "total", label: t.total, value: summary.total },
        { key: "ok", label: t.ok, value: summary.ok },
        { key: "warning", label: t.warning, value: summary.warning },
        { key: "violation", label: t.violation, value: summary.violation },
        { key: "not_verified", label: t.notVerified, value: summary.not_verified },
      ]}
    />
  );
}

function ComplianceRowView({ row }: { row: ComplianceRow }) {
  const t = useT().requirements;
  return (
    <tr>
      <Td className="align-top">
        {row.verdict ? (
          <VerdictPill verdict={row.verdict} />
        ) : (
          <Pill tone="neutral">{t.notVerified}</Pill>
        )}
      </Td>
      {/* 文は折り返す（表のセルは既定で折り返さないので、長い文が隣の列に重なる）。出典は ID の横に小さく添える */}
      <Td className="w-[34%] min-w-64 whitespace-normal align-top">
        <span className="block font-mono text-[12px] text-ink-3">
          {row.id}
          <span className="ml-2 text-ink-4">{row.source ?? t.noSource}</span>
        </span>
        <span className="block">{localized(row.statement, row.statement_en)}</span>
      </Td>
      <Td className="num whitespace-nowrap align-top">{row.constraint.text}</Td>
      {/* 確かめる先の下に、判定したケースを添える */}
      <Td className="align-top">
        <VerifiedBy verify={row.verify} input={row.input} />
        <Cases row={row} />
      </Td>
      <Td className="w-[30%] min-w-56 whitespace-normal align-top text-ink-2">
        <Result row={row} />
      </Td>
    </tr>
  );
}

/** 判定したケース。行の結果を決めたケース（`decisive`）は濃く見せる（ケースが 1 つなら区別しない） */
function Cases({ row }: { row: ComplianceRow }) {
  const t = useT().requirements;
  const many = row.cases.length > 1;
  return (
    <span className="block whitespace-normal text-ink-3 text-xs">
      {row.cases.map((c, i) => {
        const name = c.case ?? t.defaultCase;
        return (
          <span key={name}>
            {i > 0 && ", "}
            {many && c.decisive ? (
              <span className="font-medium text-ink-2" title={t.decidedCase}>
                {name}
              </span>
            ) : (
              name
            )}
          </span>
        );
      })}
    </span>
  );
}

/** 判定した文（と評価した値）か、確かめられなかった理由 */
function Result({ row }: { row: ComplianceRow }) {
  const t = useT().requirements;
  if (row.reason) {
    return (
      <span className="block">
        {t.reasons[row.reason] ?? row.reason}
        {row.detail && <span className="block text-ink-3">{row.detail}</span>}
      </span>
    );
  }
  return (
    <span className="block">
      {row.check && (
        <span className="block text-ink-3 text-xs" title={t.decidedBy}>
          {localized(row.check.label, row.check.label_en)}
          {row.check.scope && ` (${row.check.scope})`}
        </span>
      )}
      {localized(row.message, row.message_en)}
      {row.value && <span className="num block text-ink-3">{row.value.display}</span>}
    </span>
  );
}
