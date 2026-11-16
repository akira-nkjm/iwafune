import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CommandText, Op } from "@/api/types";
import { AddSharedCatalogDialog } from "./AddSharedCatalogDialog";

const state = vi.hoisted(() => ({
  commands: [] as (CommandText | null)[],
  ops: null as Op[] | null,
  error: null as string | null,
}));

// 打った引数を Op にするのも（from_command）、名前の規則と重複を見るのも（dry_run）サーバー。ここは渡し方だけを見る
vi.mock("@/api/ops", () => ({
  useCommandOps: (command: CommandText | null) => {
    state.commands.push(command);
    return { ops: state.ops, latest: state.ops, error: null };
  },
}));
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ error: state.error, ok: !state.error, pending: false, result: null }),
}));
vi.mock("@/ops/OpDialog", () => ({
  OpDialog: ({
    title,
    description,
    ops,
    children,
  }: {
    title: string;
    description: string;
    ops: Op[] | null;
    children: ReactNode;
  }) => (
    <div data-ops={JSON.stringify(ops)}>
      <h2>{title}</h2>
      <p>{description}</p>
      {children}
    </div>
  ),
}));

const op: Op = {
  op: "add_catalog_source",
  name: "lab",
  git: "https://example.com/catalog.git",
  rev: "main",
};

function render(open = true): string {
  return renderToStaticMarkup(<AddSharedCatalogDialog open={open} onOpenChange={() => {}} />);
}

describe("共有カタログを足すダイアログ", () => {
  beforeEach(() => {
    state.commands = [];
    state.ops = null;
    state.error = null;
  });

  it("名前・git の URL・rev の欄と、取得はしないことの説明を出す", () => {
    const html = render();
    expect(html).toContain("共有カタログを足す");
    expect(html).toContain("足すだけでは取得しません");
    expect(html).toContain('id="shared-name"');
    expect(html).toContain('id="shared-git"');
    expect(html).toContain('id="shared-rev"');
    // 欄が空の間は打った引数を送らない
    expect(state.commands).toEqual([null]);
  });

  it("service が作った Op をそのまま確定に渡し、弾かれた理由（API の文）を名前の欄に出す", () => {
    state.ops = [op];
    state.error = "カタログ `lab` は既にあります";
    const html = render();
    expect(html).toContain("add_catalog_source");
    expect(html).toContain("カタログ `lab` は既にあります");
  });

  it("閉じている間は問い合わせない", () => {
    render(false);
    expect(state.commands).toEqual([null]);
  });
});
