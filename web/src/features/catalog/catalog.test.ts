import { describe, expect, it } from "vitest";
import { catalogItemId, itemLabel, sharedCatalogAddCommand, writableCatalogs } from "./catalog";

describe("カタログ", () => {
  it("表示名が無ければ ID を出す", () => {
    expect(itemLabel({ id: "local/rw_30mNms", label: "RW 30", label_en: null })).toBe("RW 30");
    expect(itemLabel({ id: "local/rw_30mNms", label: null, label_en: null })).toBe(
      "local/rw_30mNms",
    );
  });

  it("書き込めるカタログは API の印のとおり、fune.toml の順に並べる", () => {
    const catalogs = [
      { name: "shared", editable: false, shared: null },
      { name: "local", editable: true, shared: null },
      { name: "team", editable: true, shared: null },
    ];
    expect(writableCatalogs({ catalogs })).toEqual(["local", "team"]);
    expect(
      writableCatalogs({ catalogs: [{ name: "shared", editable: false, shared: null }] }),
    ).toEqual([]);
  });

  it("品の ID はカタログ名と名前から作る", () => {
    expect(catalogItemId("local", "rw_30mNms")).toBe("local/rw_30mNms");
  });

  it("共有カタログを足す欄は打った文字列のまま catalog_source_add にし、名前か URL が空なら送らない", () => {
    expect(sharedCatalogAddCommand(" lab ", "https://example.com/catalog.git", "")).toEqual({
      command: "catalog_source_add",
      name: " lab ",
      git: "https://example.com/catalog.git",
      rev: "",
    });
    expect(sharedCatalogAddCommand("lab", "  ", "main")).toBeNull();
    expect(sharedCatalogAddCommand("", "https://example.com/catalog.git", "main")).toBeNull();
  });
});
