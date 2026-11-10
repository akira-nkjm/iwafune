//! 画面と CLI 向けの入出力の型（DTO）。**API の型はすべてここにあり、ts-rs で `web/src/api/gen/` に書き出す。**
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `common` | 量の表示・診断・エラーの本文 |
//! | `units` | 単位の一覧と解析 |
//! | `analysis` | 解析の一覧と実行 |
//! | `design` | 設計データの表示（プロジェクト・部品・モード表・ネット・定義・設定） |
//! | `catalog` | カタログ品の表示（値・パス・使っている部品） |
//! | `value` | 値の表示（部品・カタログ品・設定で共有） |
//! | `edit` | Op の適用と、人が打った文字列（`SetText`・`UnsetText`） |
//! | `budget` | 質量・電力・データ量のバジェット |
//! | `data` | 外部データ（DE440）の状態 |

mod analysis;
mod budget;
mod catalog;
mod common;
mod data;
mod design;
mod edit;
mod format;
pub use format::{FormatDesignRequest, FormatDesignResult};
mod units;
mod value;

pub use analysis::{
    AnalysisEntry, AnalysisRunResult, AnalysisRunStatus, CheckView, DisplayChannel, DisplaySeries,
    RunAnalysisRequest,
};
pub use budget::{BudgetRequest, BudgetRow, BudgetView};
pub use catalog::{CatalogItemView, CatalogSourceView};
pub use common::{
    CheckReport, DiagnosticCounts, DiagnosticView, ErrorEnvelope, ErrorInfo, QuantityView,
    SourceExcerpt,
};
pub use data::{DataItem, DataStatus};
pub use design::PartViewRequest;
pub use design::{
    AttrSection, AttributeDefView, CaseView, DefsView, DesignCounts, DesignView, HarnessView,
    KindDefView, ModeHeader, ModeRow, ModeTable, MountView, NetView, PartRow, PartView, PartsQuery,
    PortView, ProjectInfo, SettingView, TreeNode, TreeNodeKind, ValueAnswer, ValueQuery,
};
pub use edit::{
    ApplyRequest, ApplyResult, ChangedFile, FileChangeKind, InitResult, PlaceBodyRequest, SetText,
    UnsetText,
};
pub use fune_model::design::PartField;
pub use fune_model::design::{
    AttributeDef, Case, CatalogField, CatalogItem, Harness, KindDef, Net, NetField, NetKind, Pose,
    Source, SourceKind, UncertaintySide,
};
pub use fune_model::design::{BudgetKind, StateAssign};
pub use fune_model::design::{ViewEdge, ViewLayout, ViewNode, ViewPortSide};
pub use fune_model::path::PortRef;
pub use fune_model::resolve::{Evidence, Origin};
pub use fune_model::value::{Provenance, Quantity, Quaternion, SettingValue, Value, Vector3};
pub use fune_model::{Diagnostic, Severity};
pub use fune_ops::{ModeCell, Op, ViewNodePos};
pub use fune_projection::MissingInput;
pub use sat_analysis::spec::{AnalysisInfo, AnalysisKind, Channel, Check, Series, System, Verdict};
pub use units::{
    DimensionInfo, FormatItem, FormatRequest, FormatResponse, ParseQuantityRequest, ParsedQuantity,
    UnitInfo, UnitMapping, UnitsCatalog, UnitsQuery,
};
pub use value::{ValueData, ValueView};

pub use crate::export::{ExportError, export_ts};
