import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { RenameDefinition } from "./RenameDefinition";

describe("属性・種別の名前の変更ボタン", () => {
  it("属性と種別で別の名前のボタンを出す", () => {
    expect(
      renderToStaticMarkup(<RenameDefinition what="attribute" id="linear_density" />),
    ).toContain("属性 linear_density の名前を変える");
    expect(renderToStaticMarkup(<RenameDefinition what="kind" id="wire" />)).toContain(
      "種別 wire の名前を変える",
    );
  });
});
