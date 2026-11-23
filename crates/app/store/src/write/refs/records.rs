//! 運用エンティティ（ネット・ハーネス・解析ケース・出典・マヌーバ・通信の回線）の名前の変更に伴う参照の書き換え。
//! 参照の所在は model の索引（`Design::references` の `Referent::{Net, Harness, Case, Source, Maneuver, Link}`）で、
//! ここはファイルの移動（回線は `[link.<名前>]` のキー）と、参照元のファイルのキーの書き換えだけをする。
//! 共有カタログの品は書き換えられないので、呼ぶ前に fune-ops が拒む（ここでは `catalog_file` が誤りを返す）。

use super::defs::catalog_file;
use super::{Edits, part_file, rename_in, rename_sites, set_in, set_top};
use crate::error::StoreError;
use crate::layout;
use crate::write::doc::{replace_in_array, table_opt};
use crate::write::part::set_attr;
use fune_model::Design;
use fune_model::design::{AnalysisSystem, RefSite, Referent};

/// ネットの名前を変える（ファイルの移動と、部品のポートの `net`・ハーネスの `nets`・図のノードのキーの書き換え）。
pub fn rename_net(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::net_file(from), &layout::net_file(to))?;
    rename_sites(edits, design, Referent::Net(from), to, None)
}

/// ハーネス区間の名前を変える（ファイルの移動だけ。ハーネスを指すものは無い）。
pub fn rename_harness(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::harness_file(from), &layout::harness_file(to))?;
    rename_sites(edits, design, Referent::Harness(from), to, None)
}

/// 解析ケースの名前を変える（ファイルの移動と、既定のケース・要求の量化の書き換え）。
pub fn rename_case(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::case_file(from), &layout::case_file(to))?;
    rename_sites(edits, design, Referent::Case(from), to, None)
}

/// マヌーバの名前を変える（ファイルの移動と、ほかのマヌーバの `after` の書き換え）。
pub fn rename_maneuver(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::maneuver_file(from), &layout::maneuver_file(to))?;
    rename_sites(edits, design, Referent::Maneuver(from), to, None)
}

/// 出典の名前を変える（同じディレクトリの中でファイルを移し、値・カタログ品・要求の参照を書き換える）。
pub fn rename_source(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let dirs = layout::source_dir(design, from);
    edits.rename(
        &layout::source_file(design, from),
        &layout::source_path(&dirs, to),
    )?;
    rename_sites(edits, design, Referent::Source(from), to, None)
}

/// 通信の回線の名前を変える（`[link.<名前>]` のキーと、下りの回線の選択・要求の判定のキーの書き換え）。
pub fn rename_link(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let file = layout::analysis_file(AnalysisSystem::Comm);
    rename_in(edits, &file, &["link"], from, to)?;
    rename_sites(edits, design, Referent::Link(from), to, None)
}

/// 参照元 1 つを書き換える（運用エンティティの参照元だけ）。
pub(super) fn rename_record_at(
    edits: &mut Edits,
    design: &Design,
    site: &RefSite,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    let comm = || layout::analysis_file(AnalysisSystem::Comm);
    match site {
        RefSite::CommonDefaultCase => {
            set_top(edits, layout::COMMON_ANALYSIS_FILE, "default_case", to)
        }
        RefSite::RequirementCase { requirement } => {
            edits.edit(&layout::requirement_file(requirement), |doc| {
                let cases = table_opt(doc, &["quantify"]).and_then(|t| t.get_mut("cases"));
                if let Some(item) = cases {
                    replace_in_array(item, from, to);
                }
                Ok(())
            })
        }
        RefSite::RequirementSource { requirement } => {
            set_top(edits, &layout::requirement_file(requirement), "source", to)
        }
        RefSite::PartValueSource { part, attr } => {
            let value = design
                .parts
                .get(part.as_str())
                .and_then(|p| p.attr.get(attr.as_str()));
            rewrite_value(edits, &part_file(design, part)?, attr, value, (from, to))
        }
        RefSite::CatalogValueSource { item, attr } => {
            let value = design
                .catalog
                .get(item.as_str())
                .and_then(|c| c.attr.get(attr.as_str()));
            rewrite_value(edits, &catalog_file(design, item)?, attr, value, (from, to))
        }
        RefSite::CatalogItemSource { item } => {
            set_top(edits, &catalog_file(design, item)?, "source", to)
        }
        RefSite::CatalogItemDatasheet { item } => {
            set_top(edits, &catalog_file(design, item)?, "datasheet", to)
        }
        RefSite::ManeuverAfter { maneuver } => {
            set_top(edits, &layout::maneuver_file(maneuver), "after", to)
        }
        RefSite::CommDataDownlink => set_top(edits, &comm(), "data_downlink", to),
        RefSite::RequirementCheck {
            requirement,
            entity,
        } => {
            // キーの書式は `sat_checkkey`（`Verification` 経由）。名指されている種類の名前だけを書き換える
            let key = design
                .requirements
                .get(requirement.as_str())
                .and_then(|r| r.verify.check_with_renamed(*entity, from, to));
            match key {
                Some(key) => set_in(
                    edits,
                    &layout::requirement_file(requirement),
                    &["verify"],
                    "check",
                    &key,
                ),
                None => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

/// 属性の値を、出典 `from` を `to` に替えて書き直す（状態ごとのセルの出典も）。値が読めていなければ何もしない。
fn rewrite_value(
    edits: &mut Edits,
    file: &str,
    attr: &str,
    value: Option<&fune_model::value::Authored<fune_model::value::Value>>,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    match value {
        Some(v) => set_attr(edits, file, attr, &v.with_source_renamed(from, to)),
        None => Ok(()),
    }
}
