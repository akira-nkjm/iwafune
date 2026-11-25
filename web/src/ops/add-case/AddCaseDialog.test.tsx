import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { CommandText, DesignView, Op } from "@/api/types";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { AddCaseDialog } from "./AddCaseDialog";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/api/ops", () => ({
  useCommandOps: (c: CommandText | null) => ({ ops: c as unknown as Op[] | null, error: null }),
}));
vi.mock("@/api/labels", () => ({
  useEnumOptions: () => [
    { value: "nominal", label: "nominal（公称）" },
    { value: "min", label: "min（下振れ）" },
    { value: "max", label: "max（上振れ）" },
  ],
}));
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ failed: false, previewError: null, ok: true, pending: false, result: null }),
}));
vi.mock("@/ops/OpDialog", () => ({
  OpDialog: ({
    ops,
    title,
    summary,
    children,
  }: {
    ops: Op[] | null;
    title: string;
    summary: string;
    children: ReactNode;
  }) => (
    <div data-ops={JSON.stringify(ops)} data-title={title} data-summary={summary}>
      {children}
    </div>
  ),
}));

const design = {
  ...fixture,
  modes: { ...fixture.modes, modes: [{ id: "SAFE", label: "Safe" }] },
  phases: [{ id: "LEOP", label: "Commissioning" }],
  add_choices: { transceivers: [], bands: [], thermal_cases: ["cold", "hot"] },
} as unknown as DesignView;

it("必須は ID だけ。モード・フェーズ・側・熱の環境条件は選択肢から選ばせ、量と時刻は自由入力", () => {
  const html = renderToStaticMarkup(<AddCaseDialog design={design} onOpenChange={() => {}} />);
  expect(html).toContain('data-title="解析ケースを足す"');
  expect(html).toContain('data-ops="null"');
  expect(html).toContain(">SAFE（Safe）</option>");
  expect(html).toContain(">LEOP（Commissioning）</option>");
  expect(html).toContain(">max（上振れ）</option>");
  expect(html).toContain(">cold</option>");
  expect(html).toContain('id="case-beta"');
  expect(html).toContain('id="case-epoch"');
});

it("足りない入力が名指しした ID を欄に入れ、その ID で送れる状態になる", () => {
  const html = renderToStaticMarkup(
    <AddCaseDialog design={design} initialId="HOT" onOpenChange={() => {}} />,
  );
  expect(html).toContain('value="HOT"');
  expect(html).toContain('data-summary="解析ケース HOT を足す"');
  expect(html).toContain("&quot;command&quot;:&quot;case_set&quot;");
});
