import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import { keys } from "@/api/keys";
import type { DiagnosticView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { DiagnosticCard, DiagnosticItem, diagnosticKey } from "./DiagnosticItem";

const diag: DiagnosticView = {
  part: "RW_X",
  path: "RW_X.mass",
  excerpt: { first_line: 3, text: "[values]\nmass = 1" },
  severity: "error",
  code: "unit.dimension",
  message: "次元が合いません",
  target: { kind: "part", id: "RW_X" },
  file: "parts/aocs/rw_x.toml",
  line: 4,
  column: 8,
};

/** 重さの名前は API の語彙（`GET /defs` の labels.enum.severity）から引く */
function client(): QueryClient {
  const qc = new QueryClient();
  const enums = { severity: { error: { label: "エラー", label_en: "Error" } } };
  qc.setQueryData(keys.defs, { ...fixture.defs, labels: { ...fixture.defs.labels, enum: enums } });
  return qc;
}

const render = (node: ReactNode) =>
  renderToStaticMarkup(
    <QueryClientProvider client={client()}>
      <MemoryRouter>{node}</MemoryRouter>
    </QueryClientProvider>,
  );

describe("DiagnosticItem", () => {
  it("重さ（API の語彙の名前）・パス・本文・ファイルの行を 1 行に出す", () => {
    const html = render(<DiagnosticItem diagnostic={diag} />);
    expect(html.startsWith("<li")).toBe(true);
    expect(html).toContain("エラー");
    expect(html).toContain("RW_X.mass");
    expect(html).toContain("次元が合いません");
    expect(html).toContain("parts/aocs/rw_x.toml:4<");
    expect(html).not.toContain("<a ");
  });

  it("直しに行くページ（to）があれば、パスをそのページへのリンクにする", () => {
    const html = render(<DiagnosticItem diagnostic={diag} to="/design/components/RW_X" />);
    expect(html).toContain('href="/design/components/RW_X"');
  });

  it("ファイルの無い診断は行を出さない", () => {
    const html = render(<DiagnosticItem diagnostic={{ ...diag, file: null, line: null }} />);
    expect(html).not.toContain("rw_x.toml");
  });
});

describe("DiagnosticCard", () => {
  it("コード・行と列・該当行の前後を出し、該当行に印を付ける", () => {
    const html = render(<DiagnosticCard diagnostic={diag} />);
    expect(html).toContain("unit.dimension");
    expect(html).toContain("parts/aocs/rw_x.toml:4:8");
    expect(html).toContain("[values]");
    expect(html).toMatch(/class="bg-warn-bg"><span[^>]*>4<\/span>mass = 1/);
  });
});

describe("diagnosticKey", () => {
  it("同じ規則・対象・位置・本文なら同じキー、どれかが違えば別のキー", () => {
    expect(diagnosticKey(diag)).toBe(diagnosticKey({ ...diag }));
    expect(diagnosticKey(diag)).not.toBe(diagnosticKey({ ...diag, line: 5 }));
    expect(diagnosticKey(diag)).not.toBe(diagnosticKey({ ...diag, column: null }));
    expect(diagnosticKey(diag)).not.toBe(diagnosticKey({ ...diag, message: "別" }));
  });
});
