//! 名前の変更・削除に伴う参照の書き換え。**どこがその ID を参照しているかは model の索引（`Design::references`・
//! [`RefSite`]）が決め、ここは参照元のファイル（`layout`）とキーを書き換えるだけ。**

use super::Edits;
use super::doc::{rename_key, replace_in_array, set, string, table_opt};
use super::part::{rename_mount_target, set_port};
use crate::error::StoreError;
use crate::layout;
use fune_model::Design;
use fune_model::design::{AnalysisSystem, RefSite, Referent};
use fune_model::path::PortRef;
use toml_edit::Item;

mod defs;

pub use defs::{rename_attribute, rename_kind};

/// 部品の名前の変更で移ったファイル（部品のファイルと、組立品のディレクトリごと移した子部品のファイル）。
struct Moved {
    file: (String, String),
    dir: (String, String),
}

impl Moved {
    /// 移る前のファイルのパス → 移った先。
    fn after(&self, file: String) -> String {
        if file == self.file.0 {
            return self.file.1.clone();
        }
        match file.strip_prefix(&self.dir.0) {
            Some(rest) => format!("{}{rest}", self.dir.1),
            None => file,
        }
    }
}

/// 部品の名前を変える（ファイルと組立品のディレクトリの移動・参照の書き換え）。
pub fn rename_part(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let old_file = part_file(design, from)?;
    let dir = old_file
        .trim_end_matches(&format!("{from}.toml"))
        .to_string();
    let new_file = format!("{dir}{to}.toml");
    edits.rename(&old_file, &new_file)?;
    edits.rename_dir(&format!("{dir}{from}"), &format!("{dir}{to}"))?;
    let moved = Moved {
        file: (old_file, new_file),
        dir: (format!("{dir}{from}/"), format!("{dir}{to}/")),
    };
    rename_sites(edits, design, Referent::Part(from), to, Some(&moved))
}

/// グループの名前を変える。`parts/` の下のディレクトリを移し、参照（配分のキー・
/// 図の配置のキー）を書き換える。部品はディレクトリごと移るので、中の部品の参照は変わらない。
pub fn rename_group(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let node = design
        .node(from)
        .ok_or_else(|| StoreError::Missing { path: from.into() })?;
    let dirs = layout::dirs_for_parent(design, Some(&node));
    let parent: String = dirs[..dirs.len().saturating_sub(1)]
        .iter()
        .map(|d| format!("{d}/"))
        .collect();
    edits.rename_dir(
        &format!("{}/{parent}{from}", layout::PARTS_DIR),
        &format!("{}/{parent}{to}", layout::PARTS_DIR),
    )?;
    rename_sites(edits, design, Referent::Group(from), to, None)
}

/// モードの名前を変える（ファイルの移動・ケースと解析の設定の参照の書き換え）。
pub fn rename_mode(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::mode_file(from), &layout::mode_file(to))?;
    rename_sites(edits, design, Referent::Mode(from), to, None)
}

/// カタログ品の名前を変える（ファイルの移動と、部品の `catalog`・`candidates`、ハーネスの `wire` の書き換え）。
pub fn rename_catalog(
    edits: &mut Edits,
    design: &Design,
    (from, from_file): (&str, &str),
    (to, to_file): (&str, &str),
) -> Result<(), StoreError> {
    edits.rename(from_file, to_file)?;
    rename_sites(edits, design, Referent::Catalog(from), to, None)
}

/// 地上局の名前を変える（ファイルの移動と、通信の解析の `stations` の書き換え）。
pub fn rename_station(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::ground_file(from), &layout::ground_file(to))?;
    rename_sites(edits, design, Referent::Station(from), to, None)
}

/// 部品の削除に伴い、モードの行と配分を消す。ほかの参照（ネットの端・取付・解析の設定）は残すので、
/// 参照先が無くなれば検査の誤りになり、削除の計画が拒まれる。
pub fn forget_part(edits: &mut Edits, design: &Design, id: &str) -> Result<(), StoreError> {
    for site in design.references(Referent::Part(id)) {
        match site {
            RefSite::ModeRow { mode } => {
                super::entities::set_mode_row(edits, &layout::mode_file(&mode), id, false, None)?;
                super::entities::set_mode_row(edits, &layout::mode_file(&mode), id, true, None)?;
            }
            RefSite::Allocation { kind } => edits.edit(layout::ALLOCATION_FILE, |doc| {
                if let Some(t) = table_opt(doc, &[kind.as_str(), "alloc"]) {
                    t.remove(id);
                }
                Ok(())
            })?,
            _ => {}
        }
    }
    Ok(())
}

/// ネットの削除に伴い、部品のポートとハーネスから外す。
pub fn forget_net(edits: &mut Edits, design: &Design, net: &str) -> Result<(), StoreError> {
    for site in design.references(Referent::Net(net)) {
        match site {
            RefSite::PortNet { part, port } => {
                set_port(edits, &part_file(design, &part)?, &port, None)?;
            }
            RefSite::HarnessNet { harness } => {
                edits.edit(&layout::harness_file(&harness), |doc| {
                    if let Some(array) = doc.get_mut("nets").and_then(Item::as_array_mut) {
                        array.retain(|v| v.as_str() != Some(net));
                    }
                    Ok(())
                })?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// `referent` の参照元をすべて `to` に書き換える。
fn rename_sites(
    edits: &mut Edits,
    design: &Design,
    referent: Referent,
    to: &str,
    moved: Option<&Moved>,
) -> Result<(), StoreError> {
    let from = match referent {
        Referent::Part(id)
        | Referent::Group(id)
        | Referent::Mode(id)
        | Referent::Net(id)
        | Referent::Catalog(id)
        | Referent::CatalogSource(id)
        | Referent::Station(id)
        | Referent::Kind(id)
        | Referent::Attribute(id) => id,
    };
    for site in design.references(referent) {
        rename_at(edits, design, &site, (from, to), moved)?;
    }
    Ok(())
}

/// 参照元 1 つを書き換える（参照元の置き場所で 3 つに分ける）。
fn rename_at(
    edits: &mut Edits,
    design: &Design,
    site: &RefSite,
    names: (&str, &str),
    moved: Option<&Moved>,
) -> Result<(), StoreError> {
    match site {
        RefSite::ModeRow { .. }
        | RefSite::NetPort { .. }
        | RefSite::HarnessPort { .. }
        | RefSite::MountOn { .. }
        | RefSite::Allocation { .. }
        | RefSite::ViewNode { .. } => rename_key_at(edits, design, site, names, moved),
        RefSite::CommLink { .. }
        | RefSite::CommLinkAntenna { .. }
        | RefSite::ThermalNode { .. }
        | RefSite::ThermalSurface
        | RefSite::ThermalHeatNode
        | RefSite::CaseMode { .. }
        | RefSite::PowerModeShare
        | RefSite::PowerSequence { .. }
        | RefSite::PowerNamedSequence { .. }
        | RefSite::PowerBaseMode
        | RefSite::AttitudeLaw
        | RefSite::CommStation
        | RefSite::ThermalCaseMode { .. } => rename_setting_at(edits, site, names),
        // 判定のキーは解析が持つ書式なので書き換えない（名前の変更は ops が先に拒む）
        RefSite::RequirementCheck { .. } => Ok(()),
        RefSite::PortNet { .. }
        | RefSite::HarnessNet { .. }
        | RefSite::PartCatalog { .. }
        | RefSite::PartCandidate { .. }
        | RefSite::HarnessWire { .. }
        | RefSite::ManeuverThruster { .. }
        | RefSite::ManeuverMode { .. } => rename_value_at(edits, design, site, names),
        RefSite::PartKind { .. }
        | RefSite::CatalogKind { .. }
        | RefSite::KindExtends { .. }
        | RefSite::KindAttribute { .. }
        | RefSite::PartAttribute { .. }
        | RefSite::CatalogAttribute { .. } => defs::rename_def_at(edits, design, site, names),
    }
}

/// 表のキー・ポートの参照・取付先に書かれた部品とグループの ID（モード・ネット・ハーネス・部品・配分・図・fune.toml）。
fn rename_key_at(
    edits: &mut Edits,
    design: &Design,
    site: &RefSite,
    (from, to): (&str, &str),
    moved: Option<&Moved>,
) -> Result<(), StoreError> {
    match site {
        RefSite::ModeRow { mode } => {
            rename_in(edits, &layout::mode_file(mode), &["states"], from, to)?;
            rename_in(edits, &layout::mode_file(mode), &["eclipse"], from, to)
        }
        RefSite::NetPort { net, end, port } => set_top(
            edits,
            &layout::net_file(net),
            end.as_str(),
            &owner(port, to),
        ),
        RefSite::HarnessPort { harness, end, port } => set_top(
            edits,
            &layout::harness_file(harness),
            end.as_str(),
            &owner(port, to),
        ),
        RefSite::MountOn { part } => {
            let file = part_file(design, part)?;
            let file = moved.map_or_else(|| file.clone(), |m| m.after(file.clone()));
            rename_mount_target(edits, &file, to)
        }
        RefSite::Allocation { kind } => rename_in(
            edits,
            layout::ALLOCATION_FILE,
            &[kind.as_str(), "alloc"],
            from,
            to,
        ),
        RefSite::ViewNode { view } => {
            rename_in(edits, &layout::view_file(view), &["nodes"], from, to)
        }
        _ => Ok(()),
    }
}

/// 解析の設定（`analysis/<系>.toml`）とケースに書かれた部品・モード・地上局の ID。
fn rename_setting_at(
    edits: &mut Edits,
    site: &RefSite,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    let power = || layout::analysis_file(AnalysisSystem::Power);
    let thermal = || layout::analysis_file(AnalysisSystem::Thermal);
    match site {
        RefSite::CommLink { link } => set_in(
            edits,
            &layout::analysis_file(AnalysisSystem::Comm),
            &["link", link],
            "part",
            to,
        ),
        RefSite::CommLinkAntenna { link } => set_in(
            edits,
            &layout::analysis_file(AnalysisSystem::Comm),
            &["link", link],
            "antenna",
            to,
        ),
        RefSite::ThermalNode { node } => edits.edit(&thermal(), |doc| {
            if let Some(item) = table_opt(doc, &["nodes"]).and_then(|t| t.get_mut(node)) {
                replace_in_array(item, from, to);
            }
            Ok(())
        }),
        RefSite::ThermalSurface => rename_in(edits, &thermal(), &["surface"], from, to),
        RefSite::ThermalHeatNode => rename_in(edits, &thermal(), &["heat_node"], from, to),
        RefSite::CaseMode { case } => set_top(edits, &layout::case_file(case), "mode", to),
        RefSite::PowerModeShare => rename_in(edits, &power(), &["mode_share"], from, to),
        RefSite::PowerSequence { index } => edits.edit(&power(), |doc| {
            let segment = doc
                .get_mut("sequence")
                .and_then(Item::as_array_of_tables_mut)
                .and_then(|a| a.get_mut(*index));
            if let Some(t) = segment {
                set(t, "mode", string(to));
            }
            Ok(())
        }),
        RefSite::PowerNamedSequence { sequence, index } => edits.edit(&power(), |doc| {
            let segment = table_opt(doc, &["sequences", sequence])
                .and_then(|t| t.get_mut("segments"))
                .and_then(Item::as_array_of_tables_mut)
                .and_then(|a| a.get_mut(*index));
            if let Some(t) = segment {
                set(t, "mode", string(to));
            }
            Ok(())
        }),
        RefSite::PowerBaseMode => set_top(edits, &power(), "base_mode", to),
        RefSite::AttitudeLaw => rename_in(
            edits,
            &layout::analysis_file(AnalysisSystem::Aocs),
            &["law"],
            from,
            to,
        ),
        RefSite::ThermalCaseMode { case } => set_in(edits, &thermal(), &["case", case], "mode", to),
        RefSite::CommStation => replace_in(
            edits,
            &layout::analysis_file(AnalysisSystem::Comm),
            "stations",
            from,
            to,
        ),
        _ => Ok(()),
    }
}

/// 部品とハーネスの値に書かれたネット・カタログ品の参照（ポートの `net`・`nets`・`catalog`・`candidates`・`wire`）。
fn rename_value_at(
    edits: &mut Edits,
    design: &Design,
    site: &RefSite,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    match site {
        RefSite::PortNet { part, port } => {
            set_port(edits, &part_file(design, part)?, port, Some(to))
        }
        RefSite::HarnessNet { harness } => {
            replace_in(edits, &layout::harness_file(harness), "nets", from, to)
        }
        RefSite::PartCatalog { part } => set_top(edits, &part_file(design, part)?, "catalog", to),
        RefSite::PartCandidate { part } => {
            replace_in(edits, &part_file(design, part)?, "candidates", from, to)
        }
        RefSite::HarnessWire { harness } => {
            set_top(edits, &layout::harness_file(harness), "wire", to)
        }
        RefSite::ManeuverThruster { maneuver } => {
            set_top(edits, &layout::maneuver_file(maneuver), "thruster", to)
        }
        RefSite::ManeuverMode { maneuver } => set_in(
            edits,
            &layout::maneuver_file(maneuver),
            &["constraints"],
            "mode",
            to,
        ),
        _ => Ok(()),
    }
}
/// 部品のファイル（無ければ誤り）。
fn part_file(design: &Design, id: &str) -> Result<String, StoreError> {
    layout::part_file(design, id).ok_or_else(|| StoreError::Missing { path: id.into() })
}

/// 部品 `to` に替えたポートの参照の書き方（`Display`）。
fn owner(port: &PortRef, to: &str) -> String {
    PortRef::new(to, port.port.as_str()).to_string()
}

/// 見出しの文字列のキーを書く。
fn set_top(edits: &mut Edits, file: &str, key: &str, value: &str) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        set(doc.as_table_mut(), key, string(value));
        Ok(())
    })
}

/// 表 `table` の文字列のキーを書く（表が無ければ何もしない）。
fn set_in(
    edits: &mut Edits,
    file: &str,
    table: &[&str],
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        if let Some(t) = table_opt(doc, table) {
            set(t, key, string(value));
        }
        Ok(())
    })
}

/// 表 `table` のキーの名前を変える。
fn rename_in(
    edits: &mut Edits,
    file: &str,
    table: &[&str],
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        if let Some(t) = table_opt(doc, table) {
            rename_key(t, from, to);
        }
        Ok(())
    })
}

/// 見出しの文字列の配列 `key` の要素を置き換える。
fn replace_in(
    edits: &mut Edits,
    file: &str,
    key: &str,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        if let Some(item) = doc.get_mut(key) {
            replace_in_array(item, from, to);
        }
        Ok(())
    })
}
