//! ディレクトリの規約（どのエンティティがどこに置かれるか。docs/model.md §6・§11）。
//!
//! - `parts/` の下のディレクトリは、同じ名前のファイルが無ければ Group、あればその部品（物理的な組立品）の子。
//! - `catalog/`（`[[catalog]] path = …`）の下のディレクトリは置き場所だけ（ID はファイル名。`layout::catalog`）。
//! - `sources/` の下のディレクトリも置き場所だけ（`layout::source`。置き場所の規則はカタログと同じ `layout::subdir`）。
//! - `nets/`・`harness/`・`modes/` などは平らに並べる。ディレクトリが親の意味を持つのは `parts/` だけ。
//! - `.` で始まる名前・`.fune/`・`results/`・`exports/` は読まない。`view/` は見た目だけの情報（整数の座標）。

use crate::error::StoreError;
use fune_model::Target;
use fune_model::design::{AnalysisSystem, Design, EntityKey, NodeRef};
use fune_model::path::{ANALYSIS_COMMON, SettingPath, SettingScope};

pub const PROJECT_FILE: &str = "fune.toml";
pub const ALLOCATION_FILE: &str = "budget/allocation.toml";
pub const POLICY_FILE: &str = "budget/policy.toml";
pub const ATTRIBUTE_DIR: &str = "defs/attribute";
pub const KIND_DIR: &str = "defs/kind";
pub const PARTS_DIR: &str = "parts";
pub const MODES_DIR: &str = "modes";
pub const NETS_DIR: &str = "nets";
pub const HARNESS_DIR: &str = "harness";
pub const CASES_DIR: &str = "cases";
pub const SOURCES_DIR: &str = "sources";
/// マヌーバ（`maneuvers/<ID>.toml`）。
pub const MANEUVERS_DIR: &str = "maneuvers";
/// 規約にあるが まだ読まないディレクトリ（#9 のミッション・要求・表データ）。
pub const RESERVED_DIRS: &[&str] = &["mission", "requirements", "tables"];
pub const CONDITIONS_DIR: &str = "conditions";
pub const GROUND_DIR: &str = "conditions/ground";
pub const ANALYSIS_DIR: &str = "analysis";
/// 系に属さない解析の共通設定。
pub const COMMON_ANALYSIS_FILE: &str = "analysis/common.toml";
/// 内部の作業ディレクトリ（ロック・キャッシュ）。
pub const WORK_DIR: &str = ".fune";
/// 共有カタログの固定（`[[catalog]] git = …` の解決した commit。`fune catalog fetch`・`update` が書き、git で管理する）。
pub const LOCK_FILE: &str = "catalog.lock";
/// 共有カタログのキャッシュの置き場所（作業ディレクトリの下。`<名前>/repo.git` と `<名前>/<commit>/`）。
pub const SHARED_CATALOG_ROOT: &str = ".fune/catalogs";
/// 共有カタログのリポジトリのうち読むディレクトリ（`catalog/<ID>.toml` が品、`sources/<ID>.toml` が出典）。
pub const SHARED_CATALOG_DIRS: &[&str] = &[SHARED_ITEMS_DIR, SOURCES_DIR];
/// 共有カタログのリポジトリの品のディレクトリ。
pub const SHARED_ITEMS_DIR: &str = "catalog";
/// 読まないディレクトリ。
pub const IGNORED_DIRS: &[&str] = &["results", "exports", "target", "node_modules"];
/// 図の配置（見た目だけ。`view/<名前>.toml`）。
pub const VIEW_DIR: &str = "view";
/// `conditions/` の直下のファイル。
pub const CONDITIONS_FILES: &[&str] = &["orbit", "launch"];

/// ファイルの役割。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRole {
    Project,
    Allocation,
    Policy,
    Attribute(String),
    Kind(String),
    /// `dirs` はカタログのディレクトリからのディレクトリの列（置き場所だけで、ID にも参照にも入らない）
    Catalog {
        source: String,
        dirs: Vec<String>,
        id: String,
    },
    /// `dirs` は `parts/` からのディレクトリの列
    Part {
        dirs: Vec<String>,
        id: String,
    },
    Mode(String),
    Net(String),
    Harness(String),
    Case(String),
    /// `dirs` は `sources/` からのディレクトリの列（置き場所だけで、ID にも参照にも入らない）
    Source {
        dirs: Vec<String>,
        id: String,
    },
    Maneuver(String),
    /// `view/<名前>.toml`
    View(String),
    /// 規約にあるが まだ読まない（#9 用の `mission/`・`requirements/`・`tables/`）
    Reserved,
    /// `conditions/orbit.toml`・`conditions/launch.toml`
    Conditions(String),
    Ground(String),
    /// `analysis/<系>.toml`（系の一覧は [`AnalysisSystem`]）
    Analysis(AnalysisSystem),
    /// 系に属さない共通解析設定。
    CommonAnalysis,
    /// 規約の外（読み飛ばす。check が warning を出す）
    Other,
}

/// ID → 役割。
type RoleOf = fn(String) -> FileRole;

/// 相対パス（`/` 区切り）の役割を決める。`catalogs` は (カタログ名, ディレクトリ) の列。
pub fn classify(rel: &str, catalogs: &[(String, String)]) -> FileRole {
    let Some(stem_path) = rel.strip_suffix(".toml") else {
        return FileRole::Other;
    };
    match rel {
        PROJECT_FILE => return FileRole::Project,
        COMMON_ANALYSIS_FILE => return FileRole::CommonAnalysis,
        ALLOCATION_FILE => return FileRole::Allocation,
        POLICY_FILE => return FileRole::Policy,
        _ => {}
    }
    if let Some(role) = catalog_role(stem_path, catalogs) {
        return role;
    }
    if let Some((dirs, id)) = split_below(stem_path, SOURCES_DIR) {
        return FileRole::Source { dirs, id };
    }
    if let Some(name) = flat_child(stem_path, ANALYSIS_DIR) {
        return name
            .parse::<AnalysisSystem>()
            .map_or(FileRole::Other, FileRole::Analysis);
    }
    let flat: [(&str, RoleOf); 9] = [
        (ATTRIBUTE_DIR, FileRole::Attribute),
        (KIND_DIR, FileRole::Kind),
        (MODES_DIR, FileRole::Mode),
        (NETS_DIR, FileRole::Net),
        (HARNESS_DIR, FileRole::Harness),
        (CASES_DIR, FileRole::Case),
        (MANEUVERS_DIR, FileRole::Maneuver),
        (GROUND_DIR, FileRole::Ground),
        (VIEW_DIR, FileRole::View),
    ];
    for (dir, make) in flat {
        if let Some(id) = flat_child(stem_path, dir) {
            return make(id);
        }
    }
    if let Some(id) = flat_child(stem_path, CONDITIONS_DIR)
        && CONDITIONS_FILES.contains(&id.as_str())
    {
        return FileRole::Conditions(id);
    }
    if RESERVED_DIRS
        .iter()
        .any(|d| stem_path.starts_with(&format!("{d}/")))
    {
        return FileRole::Reserved;
    }
    if let Some(rest) = stem_path.strip_prefix("parts/") {
        let mut segments: Vec<String> = rest.split('/').map(str::to_string).collect();
        if let Some(id) = segments.pop()
            && !id.is_empty()
        {
            return FileRole::Part { dirs: segments, id };
        }
    }
    FileRole::Other
}

/// `dir/<id>`（入れ子なし）なら id。
fn flat_child(stem_path: &str, dir: &str) -> Option<String> {
    let rest = stem_path.strip_prefix(dir)?.strip_prefix('/')?;
    (!rest.is_empty() && !rest.contains('/')).then(|| rest.to_string())
}

/// 役割 → 位置を引くためのエンティティの名前。
pub fn entity_key(role: &FileRole) -> Option<EntityKey> {
    Some(match role {
        FileRole::Project => EntityKey::Project,
        FileRole::Allocation => EntityKey::Allocation,
        FileRole::Policy => EntityKey::Policy,
        FileRole::Attribute(id) => EntityKey::Attribute(id.clone()),
        FileRole::Kind(id) => EntityKey::Kind(id.clone()),
        FileRole::Catalog { source, id, .. } => EntityKey::Catalog(format!("{source}/{id}")),
        FileRole::Part { id, .. } => EntityKey::Part(id.clone()),
        FileRole::Mode(id) => EntityKey::Mode(id.clone()),
        FileRole::Net(id) => EntityKey::Net(id.clone()),
        FileRole::Harness(id) => EntityKey::Harness(id.clone()),
        FileRole::Case(id) => EntityKey::Case(id.clone()),
        FileRole::Source { id, .. } => EntityKey::Source(id.clone()),
        FileRole::Maneuver(id) => EntityKey::Maneuver(id.clone()),
        FileRole::View(id) => EntityKey::View(id.clone()),
        FileRole::Reserved => return None,
        FileRole::Conditions(id) => EntityKey::Setting(SettingPath::conditions(&[id])),
        FileRole::Ground(id) => EntityKey::Setting(SettingPath::conditions(&["ground", id])),
        FileRole::Analysis(system) => EntityKey::Setting(system.setting_path()),
        FileRole::CommonAnalysis => EntityKey::Setting(SettingPath::analysis_common()),
        FileRole::Other => return None,
    })
}

/// 役割 → そのファイルが書いているものの対象（読み込みの診断に付ける）。`fune.toml` と規約の外は None。
pub fn file_target(role: &FileRole) -> Option<Target> {
    Some(match role {
        FileRole::Allocation => SettingPath::budget(&[]).into(),
        FileRole::Policy => SettingPath::budget(&["policy"]).into(),
        FileRole::Attribute(id) => Target::AttributeDef { id: id.clone() },
        FileRole::Kind(id) => Target::KindDef { id: id.clone() },
        FileRole::Catalog { source, id, .. } => Target::Catalog {
            id: format!("{source}/{id}"),
            attr: None,
        },
        FileRole::Part { id, .. } => Target::Part { id: id.clone() },
        FileRole::Mode(id) => Target::mode(id, None),
        FileRole::Net(id) => Target::net(id, None),
        FileRole::Harness(id) => Target::harness(id, None),
        FileRole::Case(id) => Target::case(id, None),
        FileRole::Source { id, .. } => Target::Source { id: id.clone() },
        FileRole::Maneuver(id) => Target::maneuver(id, None),
        FileRole::View(id) => Target::view(id, None),
        FileRole::Conditions(id) => SettingPath::conditions(&[id]).into(),
        FileRole::Ground(id) => SettingPath::conditions(&["ground", id]).into(),
        FileRole::Analysis(system) => system.setting_path().into(),
        FileRole::CommonAnalysis => SettingPath::analysis_common().into(),
        FileRole::Project | FileRole::Reserved | FileRole::Other => return None,
    })
}

/// 製品ツリーの親から、`parts/` の下のディレクトリの列を作る。
pub fn dirs_for_parent(design: &Design, parent: Option<&NodeRef>) -> Vec<String> {
    let Some(parent) = parent else {
        return Vec::new();
    };
    let mut dirs: Vec<String> = design
        .ancestors(parent)
        .iter()
        .map(|n| n.id().to_string())
        .collect();
    dirs.reverse();
    dirs.push(parent.id().to_string());
    dirs
}

/// `parts/<dirs…>/<id>.toml`。
pub fn part_path(dirs: &[String], id: &str) -> String {
    let mut path = String::from(PARTS_DIR);
    for d in dirs {
        path.push('/');
        path.push_str(d);
    }
    format!("{path}/{id}.toml")
}

/// 部品のファイル（親の連鎖から作る）。
pub fn part_file(design: &Design, id: &str) -> Option<String> {
    let part = design.parts.get(id)?;
    Some(part_path(
        &dirs_for_parent(design, part.parent.as_ref()),
        id,
    ))
}

/// 組立品の子を置くディレクトリ（`parts/…/<id>`）。
pub fn assembly_dir(design: &Design, id: &str) -> Option<String> {
    part_file(design, id).map(|f| f.trim_end_matches(".toml").to_string())
}

pub fn mode_file(id: &str) -> String {
    format!("{MODES_DIR}/{id}.toml")
}
pub fn net_file(id: &str) -> String {
    format!("{NETS_DIR}/{id}.toml")
}
pub fn harness_file(id: &str) -> String {
    format!("{HARNESS_DIR}/{id}.toml")
}
pub fn case_file(id: &str) -> String {
    format!("{CASES_DIR}/{id}.toml")
}
pub fn maneuver_file(id: &str) -> String {
    format!("{MANEUVERS_DIR}/{id}.toml")
}
pub fn attribute_file(id: &str) -> String {
    format!("{ATTRIBUTE_DIR}/{id}.toml")
}
pub fn kind_file(id: &str) -> String {
    format!("{KIND_DIR}/{id}.toml")
}
/// 系の解析の設定のファイル（`analysis/<系>.toml`）。
pub fn analysis_file(system: AnalysisSystem) -> String {
    format!("{ANALYSIS_DIR}/{system}.toml")
}
pub fn view_file(name: &str) -> String {
    format!("{VIEW_DIR}/{name}.toml")
}

/// 共有カタログの commit の中身を置くディレクトリ（ルートからの相対パス。`.fune/catalogs/<名前>/<commit>`）。
pub fn shared_catalog_dir(name: &str, commit: &str) -> String {
    format!("{SHARED_CATALOG_ROOT}/{name}/{commit}")
}

/// 共有カタログの取得元の写し（bare リポジトリ。`.fune/catalogs/<名前>/repo.git`）。
pub fn shared_catalog_mirror(name: &str) -> String {
    format!("{SHARED_CATALOG_ROOT}/{name}/repo.git")
}

/// キャッシュのパスに使える名前と commit か（名前は ID の規則、commit は 16 進数。`..` などでキャッシュの外を指さない）。
pub fn is_cache_key(name: &str, commit: &str) -> bool {
    fune_model::id::validate_entity_id(name).is_ok()
        && (7..=64).contains(&commit.len())
        && commit.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 設定のパス → (ファイル, ファイル内のキーの列)。区切りが空のパス・ファイルに当たらないパス・キーが空になるパスは `StoreError::SettingPath`。
pub fn setting_location(path: &SettingPath) -> Result<(String, Vec<String>), StoreError> {
    let bad = || StoreError::SettingPath {
        path: path.to_string(),
    };
    let s = &path.segments;
    let (file, keys) = match path.scope {
        SettingScope::Conditions if s.first().map(String::as_str) == Some("ground") => {
            let id = s.get(1).ok_or_else(bad)?;
            (format!("{GROUND_DIR}/{id}.toml"), &s[2.min(s.len())..])
        }
        SettingScope::Conditions => match s.split_first() {
            Some((file, rest)) if CONDITIONS_FILES.contains(&file.as_str()) => {
                (format!("{CONDITIONS_DIR}/{file}.toml"), rest)
            }
            _ => return Err(bad()),
        },
        SettingScope::Analysis if s.first().map(String::as_str) == Some(ANALYSIS_COMMON) => {
            (COMMON_ANALYSIS_FILE.to_string(), &s[1..])
        }
        SettingScope::Analysis => match s.first().map(|f| f.parse::<AnalysisSystem>()) {
            Some(Ok(system)) => (analysis_file(system), &s[1..]),
            _ => return Err(bad()),
        },
        SettingScope::Budget => match s.split_first() {
            Some((head, rest)) if head == "policy" => (POLICY_FILE.to_string(), rest),
            Some(_) => (ALLOCATION_FILE.to_string(), &s[..]),
            None => return Err(bad()),
        },
    };
    if keys.is_empty() {
        return Err(bad());
    }
    Ok((file, keys.to_vec()))
}

/// 設定のパスが書かれている位置（ファイルと行。行はキーが書かれていれば）。パスがどのファイルにも当たらなければ None。
pub fn locate_setting(design: &Design, path: &SettingPath) -> Option<fune_model::design::Loc> {
    let (file, keys) = setting_location(path).ok()?;
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    Some(design.source_map.locate_in_file(&file, &keys))
}

mod catalog;
mod source;
mod subdir;
use catalog::catalog_role;
pub use catalog::{CatalogLocation, catalog_item_dir, catalog_location};
pub use source::{source_dir, source_file, source_path};
pub use subdir::{
    dir_below, dirs_text, file_in_dir, is_cacheable, parse_subdir, shared_sub_dir, split_below,
};

#[cfg(test)]
mod tests;
