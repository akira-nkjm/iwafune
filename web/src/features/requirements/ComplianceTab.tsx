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
                      <Th>{t.source}</Th>
                      <Th>{t.verify}</Th>
                      <Th>{t.cases}</Th>
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
  const cases = row.cases.map((c) => c.case ?? t.defaultCase).join(", ");
  return (
    <tr>
      <Td>
        {row.verdict ? (
          <VerdictPill verdict={row.verdict} />
        ) : (
          <Pill tone="neutral">{t.notVerified}</Pill>
        )}
      </Td>
      <Td className="max-w-[340px]">
        <span className="block font-mono text-[12px] text-ink-3">{row.id}</span>
        <span className="block">{localized(row.statement, row.statement_en)}</span>
      </Td>
      <Td className="num whitespace-nowrap">{row.constraint.text}</Td>
      <Td mono>{row.source ?? t.noSource}</Td>
      <Td>
        <VerifiedBy verify={row.verify} />
      </Td>
      <Td className="text-ink-3">{cases}</Td>
      <Td className="max-w-[360px] text-ink-2">
        <Result row={row} />
      </Td>
    </tr>
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
      {localized(row.message, row.message_en)}
      {row.value && <span className="num block text-ink-3">{row.value.display}</span>}
    </span>
  );
}
