import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { CommandText, DesignView, Op } from "@/api/types";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { AddLinkDialog } from "./AddLinkDialog";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
// 打った引数を Op にするのも（from_command）、規則を見るのも（dry_run）サーバー。ここは欄と選択肢だけを見る
vi.mock("@/api/ops", () => ({
  useCommandOps: (c: CommandText | null) => ({ ops: c as unknown as Op[] | null, error: null }),
}));
vi.mock("@/api/labels", () => ({
  useEnumOptions: () => [
    { value: "down", label: "down（ダウンリンク）" },
    { value: "up", label: "up（アップリンク）" },
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

const design: DesignView = {
  ...fixture,
  add_choices: { transceivers: ["STX", "UHF_TRX"], bands: ["s_band", "uhf"], thermal_cases: [] },
};

it("向き・送受信機・帯域は API が返した選択肢だけを選ばせ、揃うまで確定できない（Op が無い）", () => {
  const html = renderToStaticMarkup(<AddLinkDialog design={design} onOpenChange={() => {}} />);
  expect(html).toContain('data-title="通信の回線を足す"');
  expect(html).toContain(">down（ダウンリンク）</option>");
  expect(html).toContain(">STX</option>");
  expect(html).toContain(">UHF_TRX</option>");
  expect(html).toContain(">s_band</option>");
  expect(html).toContain('data-ops="null"');
});

it("足りない入力が名指しした ID を欄に入れて開く", () => {
  const html = renderToStaticMarkup(
    <AddLinkDialog design={design} initialId="s_down" onOpenChange={() => {}} />,
  );
  expect(html).toContain('value="s_down"');
  expect(html).toContain('data-summary="通信の回線 s_down を足す"');
});

it("選べる送受信機・帯域が無いときは、理由を欄の下に出す", () => {
  const html = renderToStaticMarkup(
    <AddLinkDialog
      design={{ ...design, add_choices: { transceivers: [], bands: [], thermal_cases: [] } }}
      onOpenChange={() => {}}
    />,
  );
  expect(html).toContain("送受信機の部品がありません");
  expect(html).toContain("地上局に帯域の性能がありません");
});
