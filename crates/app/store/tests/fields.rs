//! 部品・カタログ品・ネットの見出しのフィールド: 新しいファイルの正規形と、書いたものを読むと元に戻ること。
//! キーの名前と順はモデルのフィールド名の一覧（`PartFieldName` など）が正本。

use fune_model::Design;
use fune_model::design::{CatalogItem, Maturity, Net, NetKind, Part};
use fune_model::path::PortRef;
use fune_model::value::Quantity;
use fune_store::FileSet;
use fune_store::read::design_of;
use fune_store::write::{catalog::new_catalog_text, entities::new_net_text, part::new_part_text};

const PROJECT: &str =
    "[project]\nname = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";

fn read(files: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    for (p, t) in files {
        set.insert(*p, *t);
    }
    let design = design_of(&set);
    assert!(
        design.load_diagnostics.is_empty(),
        "{:?}",
        design.load_diagnostics
    );
    design
}

fn full_part() -> Part {
    let mut part = Part::new("RW_X");
    part.label = Some("X 軸 RW".into());
    part.description = Some("予備".into());
    part.catalog = Some("local/rw".into());
    part.kind = Some("reaction_wheel".into());
    part.candidates = vec!["local/rw".into(), "local/rw2".into()];
    part.count = 2;
    part.maturity = Some(Maturity::Modified);
    part
}

#[test]
fn a_new_part_file_writes_every_field_in_order_and_reads_back() {
    let part = full_part();
    let text = new_part_text(&part);
    assert_eq!(
        text,
        "label       = \"X 軸 RW\"\ndescription = \"予備\"\ncatalog     = \"local/rw\"\nkind        = \"reaction_wheel\"\n\
         candidates  = [\"local/rw\", \"local/rw2\"]\ncount       = 2\nmaturity    = \"modified\"\n"
    );
    let design = read(&[("parts/RW_X.toml", &text)]);
    assert_eq!(design.parts["RW_X"], part);
    // 既定（員数 1・空の候補・書かない欄）は書かない
    assert_eq!(new_part_text(&Part::new("P")), "\n");
}

fn full_item() -> CatalogItem {
    CatalogItem {
        id: "local/rw".into(),
        kind: "reaction_wheel".into(),
        label: Some("RW".into()),
        label_en: Some("Wheel".into()),
        manufacturer: Some("Acme".into()),
        part_number: Some("RW-1".into()),
        maturity: Some(Maturity::OffTheShelf),
        datasheet: Some("docs/rw.pdf".into()),
        source: Some("ds_rw".into()),
        attr: Default::default(),
        ports: [("PWR".to_owned(), NetKind::Power)].into(),
    }
}

#[test]
fn a_new_catalog_file_writes_every_field_in_order_and_reads_back() {
    let item = full_item();
    let text = new_catalog_text(&item);
    assert_eq!(
        text,
        "kind         = \"reaction_wheel\"\nlabel        = \"RW\"\nlabel_en     = \"Wheel\"\nmanufacturer = \"Acme\"\n\
         part_number  = \"RW-1\"\nmaturity     = \"off_the_shelf\"\ndatasheet    = \"docs/rw.pdf\"\nsource       = \"ds_rw\"\n\n\
         [ports]\nPWR = \"power\"\n"
    );
    let design = read(&[("catalog/rw.toml", &text)]);
    assert_eq!(design.catalog["local/rw"], item);
}

#[test]
fn a_new_net_file_writes_every_field_in_order_and_reads_back() {
    let mut net = Net::new("CAN", NetKind::Data);
    net.label = Some("CAN".into());
    net.source = Some(PortRef::new("PCDU", "OUT1"));
    net.rail = Some(Quantity::new(5.0, "V"));
    net.limit = Some(Quantity::new(1.0, "A"));
    net.protocol = Some("CAN".into());
    net.bitrate = Some(Quantity::new(1.0, "Mbit/s"));
    net.master = Some(PortRef::new("OBC", "CAN1"));
    let text = new_net_text(&net);
    assert_eq!(
        text,
        "kind     = \"data\"\nlabel    = \"CAN\"\nsource   = \"PCDU.OUT1\"\nrail     = { value = 5.0, unit = \"V\" }\n\
         limit    = { value = 1.0, unit = \"A\" }\nprotocol = \"CAN\"\nbitrate  = { value = 1.0, unit = \"Mbit/s\" }\n\
         master   = \"OBC.CAN1\"\n"
    );
    let design = read(&[("nets/CAN.toml", &text)]);
    assert_eq!(design.nets["CAN"], net);
}
