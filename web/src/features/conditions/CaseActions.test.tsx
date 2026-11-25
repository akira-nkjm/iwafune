import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { CaseView } from "@/api/types";
import { CaseActions } from "./CaseActions";

// 規則（参照されているケースを消せない・名前の変更で参照を書き換える）を見るのは Op を検査するサーバー。ここはボタンの出し方だけを見る
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ failed: false, previewError: null, ok: true, pending: false, result: null }),
}));

const kase: CaseView = {
  id: "hot_imaging",
  label: "Hot imaging",
  description: null,
  modes: [],
  phase: null,
  epoch: null,
  file: "cases/hot_imaging.toml",
};

describe("解析ケースの操作", () => {
  it("名前の変更と削除のボタンを出す（ダイアログは押すまで開かない）", () => {
    const html = renderToStaticMarkup(<CaseActions kase={kase} />);
    expect(html).toContain("ケース hot_imaging の名前を変える");
    expect(html).toContain("ケース hot_imaging を消す");
    expect(html).not.toContain('role="dialog"');
  });
});
