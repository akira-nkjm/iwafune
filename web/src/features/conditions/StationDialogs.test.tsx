import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { GroundStationView } from "@/api/types";
import { AddStationButton, StationActions } from "./StationDialogs";

// 打った引数を Op にするのも（from_command）、規則を見るのも（dry_run）サーバー。ここはボタンの出し方だけを見る
vi.mock("@/api/ops", () => ({
  useCommandOps: () => ({ ops: null, latest: null, error: null }),
}));
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ failed: false, previewError: null, ok: true, pending: false, result: null }),
}));

const station: GroundStationView = {
  id: "TOKYO_GS",
  label: "Tokyo",
  file: "conditions/ground/TOKYO_GS.toml",
  group: "conditions.ground.TOKYO_GS",
  in_use: false,
};

describe("地上局の操作", () => {
  it("名前の変更と削除のボタンを出す（ダイアログは押すまで開かない）", () => {
    const html = renderToStaticMarkup(<StationActions station={station} />);
    expect(html).toContain("地上局 TOKYO_GS の名前を変える");
    expect(html).toContain("地上局 TOKYO_GS を消す");
    expect(html).not.toContain('role="dialog"');
  });

  it("解析が挙げている地上局は、削除のボタンの説明で消せない理由を示す（API の in_use）", () => {
    const html = renderToStaticMarkup(<StationActions station={{ ...station, in_use: true }} />);
    expect(html).toContain("analysis.comm.stations");
  });

  it("足すボタンを出す", () => {
    expect(renderToStaticMarkup(<AddStationButton />)).toContain("地上局を足す");
  });
});
