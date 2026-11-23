import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { CaseView, Op } from "@/api/types";
import { CasePhaseDialog } from "@/features/conditions/CasePhaseDialog";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";

const REASON = "Part `NEW` already exists";

// サーバー（dry_run）が弾いた状態。理由の文はプレビューだけが出し、欄の下には出さない（同じ文を 2 か所に出さない）
const OPS: Op[] = [{ op: "set_case_phase", case: "hot", phase: null }];

vi.mock("@/api/ops", () => ({
  // 打った引数から service が Op を作った状態
  useCommandOps: () => ({ ops: OPS, latest: OPS, error: null }),
  useDryRun: () => ({
    data: undefined,
    isFetching: false,
    error: {
      message: "rejected",
      diagnostics: [{ severity: "error", code: "id.taken", message: REASON, path: null }],
    },
  }),
  useApplyOps: () => ({ mutateAsync: async () => {}, isError: false, isPending: false }),
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
// Radix のダイアログは portal に描くので、静的な描画では中身をそのまま出す
vi.mock("@/ui/dialog", () => {
  const Pass = ({ children }: { children?: ReactNode }) => <div>{children}</div>;
  return {
    Dialog: Pass,
    DialogContent: Pass,
    DialogDescription: Pass,
    DialogFooter: Pass,
    DialogHeader: Pass,
    DialogTitle: Pass,
  };
});

const count = (haystack: string, needle: string) => haystack.split(needle).length - 1;

describe("編集のダイアログの弾かれた理由", () => {
  it("dry_run が弾いた理由は、ダイアログに 1 回だけ出る（プレビュー。欄の下には出さない）", () => {
    const kase = { id: "hot", phase: null } as unknown as CaseView;
    const html = renderToStaticMarkup(
      <CasePhaseDialog kase={kase} phases={fixture.phases} onOpenChange={() => {}} />,
    );
    expect(count(html, REASON)).toBe(1);
    // 出ている場所はプレビュー（CLI の代わりに理由を出す枠）
    expect(html).toContain("text-neg-ink");
  });
});

/** 弾かれた理由の文（`previewError`）を使ってよいのはプレビューだけ。ダイアログが欄の下に出すと、同じ文が 2 回出る。 */
describe("理由の文の置き場所", () => {
  const root = join(__dirname, "..");
  const sources = (dir: string): string[] =>
    readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return name === "gen" ? [] : sources(path);
      return /\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
    });

  it("previewError を読むのは ops/OpPreview.tsx（と定義の ops/use-op-check.ts）だけ", () => {
    const users = sources(root)
      .filter((p) => readFileSync(p, "utf8").includes("previewError"))
      .map((p) => relative(root, p))
      .sort();
    expect(users).toEqual(["ops/OpPreview.tsx", "ops/use-op-check.ts"]);
  });

  it("useOpCheck の結果の error を読む場所は無い（弾かれたかは failed、理由の文はプレビュー）", () => {
    const offenders = sources(root)
      .filter((p) => /useOpCheck\([^)]*\)\.error\b/.test(readFileSync(p, "utf8")))
      .map((p) => relative(root, p));
    expect(offenders).toEqual([]);
  });
});
