//! 定義（属性・種別）の名前の変更に伴う参照の書き換え。参照の所在は model の索引（`Design::references` の
//! `Referent::{Kind, Attribute}`）で、定義のファイルの移動と、部品・カタログ品・種別のファイルの書き換えだけをここでする。
//! 共有カタログの品は書き換えられないので、呼ぶ前に fune-ops が拒む（ここでは `catalog_file` が誤りを返す）。

use super::{Edits, part_file, rename_in, rename_sites, replace_in, set_top};
use crate::error::StoreError;
use crate::layout::{self, CatalogLocation};
use fune_model::Design;
use fune_model::design::{RefSite, Referent};

/// 属性定義の名前を変える（ファイルの移動と、種別の `required` / `optional`・部品とカタログ品の `[attr]` のキーの書き換え）。
pub fn rename_attribute(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::attribute_file(from), &layout::attribute_file(to))?;
    rename_sites(edits, design, Referent::Attribute(from), to, None)
}

/// 種別定義の名前を変える（ファイルの移動と、部品とカタログ品の `kind`・種別の `extends` の書き換え）。
pub fn rename_kind(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::kind_file(from), &layout::kind_file(to))?;
    rename_sites(edits, design, Referent::Kind(from), to, None)
}

/// 参照元 1 つを書き換える（定義の参照元だけ）。
pub(super) fn rename_def_at(
    edits: &mut Edits,
    design: &Design,
    site: &RefSite,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    match site {
        RefSite::PartKind { part } => set_top(edits, &part_file(design, part)?, "kind", to),
        RefSite::CatalogKind { item } => set_top(edits, &catalog_file(design, item)?, "kind", to),
        RefSite::KindExtends { kind } => set_top(edits, &layout::kind_file(kind), "extends", to),
        RefSite::KindAttribute { kind } => {
            let file = layout::kind_file(kind);
            replace_in(edits, &file, "required", from, to)?;
            replace_in(edits, &file, "optional", from, to)
        }
        RefSite::PartAttribute { part } => {
            rename_in(edits, &part_file(design, part)?, &["attr"], from, to)
        }
        RefSite::CatalogAttribute { item } => {
            rename_in(edits, &catalog_file(design, item)?, &["attr"], from, to)
        }
        _ => Ok(()),
    }
}

/// このプロジェクトのカタログ品のファイル（共有カタログの品・読めない参照は誤り）。
fn catalog_file(design: &Design, reference: &str) -> Result<String, StoreError> {
    match layout::catalog_location(design, reference) {
        CatalogLocation::Local { file, .. } => Ok(file),
        _ => Err(StoreError::Missing {
            path: reference.into(),
        }),
    }
}
