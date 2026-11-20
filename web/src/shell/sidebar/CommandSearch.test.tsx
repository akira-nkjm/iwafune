import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { SearchPalette } from "./CommandSearch";

describe("検索（⌘K）の中身", () => {
  it("cmdk の `Command` の中に描くので、開いても落ちない（ページと部品の候補が出る）", () => {
    const html = renderToStaticMarkup(<SearchPalette design={fixture} onGo={() => undefined} />);
    const t = getT().shell;
    expect(html).toContain(t.groupPages);
    expect(html).toContain(t.groupParts);
    expect(html).toContain(fixture.parts[0]?.id ?? "");
  });

  it("設計がまだ無くても、ページの候補は出る", () => {
    const html = renderToStaticMarkup(<SearchPalette design={undefined} onGo={() => undefined} />);
    expect(html).toContain(getT().shell.groupPages);
  });
});
