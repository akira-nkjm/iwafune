import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { nodeLabel } from "@/lib/design";
import { fixture } from "@/test/fixture";
import { PartsTree } from "./PartsTree";

function render(items = fixture.parts) {
  return renderToStaticMarkup(
    <PartsTree
      all={fixture.parts}
      items={items}
      selected={null}
      onSelect={() => {}}
      folderLabel={(id) => nodeLabel(fixture, id)}
    />,
  );
}

describe("PartsTree", () => {
  it("フォルダーの見出しは呼び手の folderLabel（製品ツリーのノードの表示名）", () => {
    const html = render();
    expect(html).toContain('aria-label="姿勢制御系"');
    expect(html).toContain('aria-label="通信系"');
  });

  it("絞り込んだ部品（items）だけを、祖先のフォルダーを開いて出す", () => {
    const rw = fixture.parts.filter((p) => p.id === "RW_X");
    const html = render(rw);
    expect(html).toContain('aria-label="姿勢制御系"');
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("RW_X");
    expect(html).not.toContain("PCDU");
    expect(html).not.toContain('aria-label="電源系"');
  });
});
