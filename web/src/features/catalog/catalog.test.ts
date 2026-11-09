import { describe, expect, it } from "vitest";
import { catalogItemId, itemLabel, writableCatalogs } from "./catalog";

describe("カタログ", () => {
  it("表示名が無ければ ID を出す", () => {
    expect(itemLabel({ id: "local/rw_30mNms", label: "RW 30", label_en: null })).toBe("RW 30");
    expect(itemLabel({ id: "local/rw_30mNms", label: null, label_en: null })).toBe(
      "local/rw_30mNms",
    );
  });

  it("書き込めるカタログは API の印のとおり、fune.toml の順に並べる", () => {
    const catalogs = [
      { name: "shared", editable: false },
      { name: "local", editable: true },
      { name: "team", editable: true },
    ];
    expect(writableCatalogs({ catalogs })).toEqual(["local", "team"]);
    expect(writableCatalogs({ catalogs: [{ name: "shared", editable: false }] })).toEqual([]);
  });

  it("品の ID はカタログ名と名前から作る", () => {
    expect(catalogItemId("local", "rw_30mNms")).toBe("local/rw_30mNms");
  });
});
