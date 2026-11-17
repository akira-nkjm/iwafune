import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { q } from "@/api/queries";
import type { CatalogItemView, DesignView, KindDefView } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { fixture } from "@/test/fixture";
import { CatalogDetail } from "./CatalogDetail";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
afterEach(() => setLang("ja"));

const kind: KindDefView = {
  id: "reaction_wheel",
  label: "リアクションホイール",
  label_en: "Reaction wheel",
  role: "reaction_wheel",
  parent: null,
  states: ["off", "on"],
  required: [],
  optional: [],
  ports: ["PWR"],
  builtin: true,
};

const design: DesignView = { ...fixture, defs: { ...fixture.defs, kinds: [kind] } };

function item(extra: Partial<CatalogItemView> = {}): CatalogItemView {
  return {
    id: "local/rw",
    scope: "local",
    name: "rw",
    editable: true,
    kind: "reaction_wheel",
    label: "RW",
    label_en: "RW",
    manufacturer: null,
    part_number: null,
    maturity: null,
    datasheet: null,
    datasheet_link: null,
    source: null,
    ports: { PWR: "power", TLM: "data" },
    states: ["off", "on"],
    values: {},
    paths: {},
    users: ["RW1", "RW2"],
    ...extra,
  } as CatalogItemView;
}

function render(it: CatalogItemView): string {
  const client = new QueryClient();
  client.setQueryData(q.defs().queryKey, {
    ...design.defs,
    labels: {
      ...design.defs.labels,
      enum: { net_kind: { power: { label: "電源", label_en: "Power" } } },
    },
  });
  return renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <CatalogDetail design={design} item={it} onSelect={() => {}} />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("カタログ品の詳細", () => {
  it("副題は種別の表示名で、ポートと状態を出す", () => {
    const html = render(item());
    expect(html).toContain("RW · リアクションホイール");
    expect(html).toContain('<section class="space-y-1.5" aria-label="ポート"');
    expect(html).toContain("PWR");
    expect(html).toContain("電源");
    expect(html).toContain('<section class="space-y-1.5" aria-label="状態"');
    expect(html).toContain(">on<");
  });

  it("ポートも状態も空なら、その区分を出さない", () => {
    const html = render(item({ ports: {}, states: [] }));
    expect(html).not.toContain('<section class="space-y-1.5" aria-label="ポート"');
    expect(html).not.toContain('<section class="space-y-1.5" aria-label="状態"');
  });

  it("読み取り専用の品はカタログ名のピルと注記を出し、編集の操作を出さない", () => {
    const html = render(item({ editable: false, scope: "lab", id: "lab/rw" }));
    expect(html).toContain("読み取り専用");
    expect(html).toContain(">lab<");
    expect(html).not.toContain("名前を変える");
    expect(render(item())).not.toContain("読み取り専用");
  });

  it("設計に追加する操作は、書き込めない共有カタログの品にも出す", () => {
    expect(render(item())).toContain('aria-label="部品として設計に追加"');
    const shared = render(item({ editable: false, scope: "lab", id: "lab/rw" }));
    expect(shared).toContain('aria-label="部品として設計に追加"');
    expect(shared).not.toContain("名前を変える");
  });

  it("使っている部品を数と一緒にリンクで出す", () => {
    const html = render(item());
    expect(html).toContain("使っている部品（2）");
    expect(html).toContain("RW1");
    expect(html).toContain("<a ");
  });
});
