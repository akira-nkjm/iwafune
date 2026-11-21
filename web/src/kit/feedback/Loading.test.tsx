import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SkeletonShell } from "./Loading";

describe("SkeletonShell（アプリ全体の骨組み）", () => {
  // プロバイダ（クエリ・ルーター・i18n）なしで描く（初回ロード中はまだ無い）
  const out = renderToStaticMarkup(<SkeletonShell />);

  it("プロバイダなしで描け、画面読み上げには読み込み中を伝える", () => {
    expect(out).toContain('role="status"');
    expect(out).toContain('aria-busy="true"');
    expect(out).toContain("sr-only");
  });

  it("サイドバーの列と本体のパネルの形のスケルトンを含む", () => {
    expect(out).toContain("w-sidebar");
    expect(out).toContain("rounded-panel");
    expect(out).toContain('data-slot="skeleton"');
  });
});
