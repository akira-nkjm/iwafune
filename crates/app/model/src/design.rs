//! Design 層の型（書かれたままの設計。docs/model.md §4）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `part` | Part・Group・NodeRef・Mount・Pose・PortBinding・CatalogItem |
//! | `defs` | AttributeDef・KindDef・BudgetKind・NetKind |
//! | `system` | Mode・Net・Harness・Case |
//! | `source` | Source・SourceKind・Distribution（出典の台帳） |
//! | `budget` | 配分とマージン方針 |
//! | `project` | `fune.toml` |
//! | `shared_catalog` | 共有カタログの固定（`catalog.lock`）と読み込みの状態の判定（`Design::shared_catalog_statuses`） |
//! | `conditions` / `analysis` | `conditions/`・`analysis/` の型付きの設定。解析の系の一覧 `AnalysisSystem` |
//! | `thermal_settings` | 熱の解析の設定（`analysis/thermal.toml`。環境条件・プリセット・解析マージン・表面材・発熱の場所） |
//! | `refs` | 参照の索引（ある ID をどのエンティティのどのキーが持っているか。`Design::references`） |
//! | `setting_dims` | 設定の量の次元の表（パスの形 → 次元。検査と射影が引く） |
//! | `settings_view` | 型付きの設定をパス記法で読む（`Design::setting`・`setting_entries`・`settings_json`） |
//! | `source_map` | エンティティ → ファイル・キー → 行 |
//! | `view` | 図の配置（見た目だけ） |

pub mod analysis;
pub mod budget;
pub mod conditions;
pub mod defs;
pub mod net;
pub mod part;
pub mod project;
pub mod refs;
pub mod setting_dims;
pub mod settings_view;
pub mod shared_catalog;
pub mod source;
pub mod source_map;
pub mod system;
pub mod thermal_settings;
pub mod view;

pub use analysis::*;
pub use budget::*;
pub use conditions::*;
pub use defs::*;
pub use part::*;
pub use project::*;
pub use refs::{HarnessEnd, NetEnd, RefSite, Referent};
pub use settings_view::SettingEntry;
pub use shared_catalog::{CatalogLock, LockedCatalog, SharedCatalogState, SharedCatalogStatus};
pub use source_map::{EntityKey, Loc, SourceMap};
pub use system::*;
pub use thermal_settings::*;
pub use view::{ViewEdge, ViewLayout, ViewNode, ViewPortSide};

use crate::chain::chain;
use crate::diagnostic::Diagnostic;
use crate::id::Id;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

/// 追跡ファイルの (path, bytes) の BLAKE3（store が計算する）。楽観ロックに使う。
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS,
)]
pub struct Revision(pub String);

impl Revision {
    /// 表示用の短い形（先頭 12 文字）。
    pub fn short(&self) -> &str {
        let end = self
            .0
            .char_indices()
            .nth(12)
            .map_or(self.0.len(), |(i, _)| i);
        &self.0[..end]
    }
}

impl std::fmt::Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 書かれたままの設計。不変のスナップショットで、編集のたびに新しいものを作る。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct Design {
    pub project: Project,
    pub budget: Budget,
    /// プロジェクト独自の属性定義（組み込みは [`Design::attribute`] が合わせて引く）
    pub attributes: BTreeMap<Id<AttributeDef>, AttributeDef>,
    /// プロジェクト独自の種別定義
    pub kinds: BTreeMap<Id<KindDef>, KindDef>,
    /// カタログ品（`"local/rw_10mNms"` → 定義）
    pub catalog: BTreeMap<CatalogRef, CatalogItem>,
    pub groups: BTreeMap<Id<Group>, Group>,
    pub parts: BTreeMap<Id<Part>, Part>,
    pub modes: BTreeMap<Id<Mode>, Mode>,
    pub nets: BTreeMap<Id<Net>, Net>,
    pub harnesses: BTreeMap<Id<Harness>, Harness>,
    pub cases: BTreeMap<Id<Case>, Case>,
    pub sources: BTreeMap<Id<Source>, Source>,
    pub conditions: ConditionsSettings,
    pub analysis: AnalysisSettings,
    /// 図の配置（`view/<名前>.toml`。見た目だけ）
    pub views: BTreeMap<String, ViewLayout>,
    /// `catalog.lock`（無ければ None。共有カタログの固定した commit）
    pub catalog_lock: Option<CatalogLock>,
    /// lock の commit のキャッシュ（`.fune/catalogs/`）を読んだ共有カタログの名前
    pub fetched_catalogs: BTreeSet<String>,
    pub revision: Revision,
    /// 読み込みのときに見つかった問題（構文・型の誤り。該当エンティティは読み飛ばされている）
    pub load_diagnostics: Vec<Diagnostic>,
    /// 位置の索引（API には出さない）
    #[serde(skip)]
    #[ts(skip)]
    pub source_map: SourceMap,
}

impl Design {
    /// 部品の種別 ID（部品に書かれた `kind`、無ければカタログ品の `kind`）。
    pub fn part_kind_id<'a>(&'a self, part: &'a Part) -> Option<&'a str> {
        part.kind.as_ref().map(Id::as_str).or_else(|| {
            part.catalog
                .as_ref()
                .and_then(|c| self.catalog.get(c))
                .map(|item| item.kind.as_str())
        })
    }

    /// 種別の状態の語彙（継承を含む。最初に見つかった空でない語彙）。
    pub fn kind_states(&self, kind: &str) -> &[String] {
        self.kind_chain(kind)
            .into_iter()
            .map(|k| k.states.as_slice())
            .find(|s| !s.is_empty())
            .unwrap_or(&[])
    }

    /// 部品の状態の語彙。
    pub fn part_states<'a>(&'a self, part: &'a Part) -> &'a [String] {
        self.part_kind_id(part)
            .map_or(&[], |kind| self.kind_states(kind))
    }

    /// 種別の役割（継承を含む）。
    pub fn kind_role(&self, kind: &str) -> Option<crate::role::Role> {
        self.kind_chain(kind).into_iter().find_map(|k| k.role)
    }

    /// Group か Part の ID が存在するか。
    pub fn node_exists(&self, id: &str) -> bool {
        self.parts.contains_key(id) || self.groups.contains_key(id)
    }

    /// ID が指すノード。
    pub fn node(&self, id: &str) -> Option<NodeRef> {
        if self.parts.contains_key(id) {
            Some(NodeRef::Part(Id::new(id)))
        } else if self.groups.contains_key(id) {
            Some(NodeRef::Group(Id::new(id)))
        } else {
            None
        }
    }

    /// ノードの親。
    pub fn parent_of(&self, node: &NodeRef) -> Option<&NodeRef> {
        match node {
            NodeRef::Group(id) => self.groups.get(id).and_then(|g| g.parent.as_ref()),
            NodeRef::Part(id) => self.parts.get(id).and_then(|p| p.parent.as_ref()),
        }
    }

    /// ノードの祖先の連鎖（親 → 最上位の順。循環していればそこで止める）。
    pub fn ancestors(&self, node: &NodeRef) -> Vec<NodeRef> {
        chain(self.parent_of(node).cloned(), NodeRef::clone, |n| {
            self.parent_of(n).cloned()
        })
        .collect()
    }

    /// カタログ品を参照しているもの（部品の ID、ハーネスは `harness/<ID>`。部品 → ハーネスの順）。
    /// 参照の所在は [`Design::references`]（削除を拒む文に使う）。
    pub fn catalog_users(&self, reference: &str) -> Vec<String> {
        let harnesses = self
            .references(Referent::Catalog(reference))
            .into_iter()
            .filter_map(|site| match site {
                RefSite::HarnessWire { harness } => Some(format!("harness/{harness}")),
                _ => None,
            });
        self.catalog_part_users(reference)
            .into_iter()
            .chain(harnesses)
            .collect()
    }

    /// カタログ品を選択中（`catalog`）か候補（`candidates`）に入れている部品の ID（ID 順）。参照の所在は [`Design::references`]。
    pub fn catalog_part_users(&self, reference: &str) -> Vec<String> {
        let mut parts: Vec<String> = self
            .references(Referent::Catalog(reference))
            .into_iter()
            .filter_map(|site| match site {
                RefSite::PartCatalog { part } | RefSite::PartCandidate { part } => Some(part),
                _ => None,
            })
            .collect();
        parts.dedup();
        parts
    }
}
