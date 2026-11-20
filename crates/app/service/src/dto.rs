//! 画面と CLI 向けの入出力の型（DTO）。**API の型はすべてここにあり、ts-rs で `web/src/api/gen/` に書き出す。**
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `common` | 量の表示・診断・エラーの本文 |
//! | `units` | 単位の一覧と解析 |
//! | `aocs_modes` | AOCS の姿勢・役割別機器・モード別解析のカード |
//! | `analysis` | 解析の一覧と実行 |
//! | `check` | 解析の判定 1 つ（結果と概要の行で共有する葉の型） |
//! | `display` | 解析表示の葉の型（名前・時系列） |
//! | `power_modes` | 電源のモードごとの電力の行き先・判定・グラフの表示定義 |
//! | `design` | 設計データの表示（プロジェクト・部品・モード表・ネット・定義・設定） |
//! | `catalog` | カタログ品の表示（値・パス・使っている部品） |
//! | `shared_catalog` | 共有カタログの取得・更新の入出力（lock の commit・変わった品と値） |
//! | `source` | 出典の台帳の表示（リンク・ファイル） |
//! | `value` | 値の表示（部品・カタログ品・設定で共有） |
//! | `edit` | Op の適用と、人が打った文字列（`SetText`・`UnsetText`） |
//! | `command` | 名詞のコマンドに打った引数（`CommandText`。`fune net set`・`fune part place`・`fune mode set` など） |
//! | `maneuver` | マヌーバの表示（`ManeuverView`）と `fune maneuver add\|set` の引数（`ManeuverText`） |
//! | `requirement` | 要求の表示（`RequirementView`）・`fune requirement add\|set` の引数（`RequirementText`）・要求適合表（`ComplianceReport`） |
//! | `budget` | 質量・電力・データ量のバジェット |
//! | `kpi` | KPI（系の代表値・判定・塗りの比・判定の数え方）と塗りの比 |
//! | `overview` | 系の概要の解析の表の行（主な結果・判定のまとまり・小さな棒グラフ・隠す表） |
//! | `net_class` | ネットの分類（電源の系統・星形の中心） |
//! | `data` | 外部データ（DE440）の状態 |

mod aocs_modes;
pub use aocs_modes::*;
mod analysis;
mod check;
pub use check::CheckView;
mod display;
pub use display::{AnalysisLabel, DisplayChannel, DisplaySeries};
mod budget;
mod catalog;
mod command;
mod common;
mod data;
mod design;
mod edit;
mod format;
mod history;
pub use format::{FormatDesignRequest, FormatDesignResult};
mod kpi;
mod maneuver;
mod requirement;
pub use kpi::{FillBasis, FillRatio, KpiDef, KpiKey, KpiTally, KpiView, OverviewKpi};
pub use maneuver::{ManeuverTargetView, ManeuverText, ManeuverView};
pub use requirement::{
    ComplianceCase, ComplianceReport, ComplianceRow, ComplianceSummary, ConstraintView,
    NotVerifiedReason, RequirementText, RequirementView,
};
mod overview;
pub use overview::{
    AnalysisOverview, OverviewBar, OverviewChart, OverviewCheckGroup, OverviewReference,
};
mod pose;
pub use net_class::{NetHub, PowerClass};
pub use pose::{BodyPoseView, MountPointView, MountView, PoseQuantities};
mod power_modes;
pub use power_modes::*;
mod shared_catalog;
pub use shared_catalog::{
    CatalogFetchRequest, CatalogItemChange, CatalogItemChangeKind, CatalogSyncEntry,
    CatalogSyncResult, CatalogUpdateRequest, CatalogValueChange,
};
mod source;
pub use source::SourceView;
mod net_class;
mod units;
mod value;

pub use analysis::{
    AnalysisCaseSource, AnalysisEntry, AnalysisInputSource, AnalysisRunResult, AnalysisRunStatus,
    PartHeat, RunAnalysisRequest,
};
pub use budget::{
    BudgetPropellant, BudgetRequest, BudgetRow, BudgetTotal, BudgetUncounted, BudgetView,
};
pub use catalog::{CatalogItemView, CatalogSourceView};
pub use command::{
    AttrAddText, CaseFromEclipseRequest, CatalogSourceAddText, CommandText, EclipseCaseAt,
    KindAddText, ModeCellText, ModeSetText, NetSetText, NetUnsetText, PlaceText,
};
pub use common::{
    CheckReport, DiagnosticCounts, DiagnosticView, ErrorEnvelope, ErrorInfo, QuantityView,
    SourceExcerpt,
};
pub use data::{DataItem, DataStatus, SpaceWeatherLookup, SpaceWeatherQuery, SpaceWeatherVersion};
pub use design::PartViewRequest;
pub use design::{
    AttrSection, AttributeDefView, CaseView, DefsView, DesignCounts, DesignView, HarnessView,
    KindDefView, ModeHeader, ModeRow, ModeSegmentView, ModeSequenceView, ModeStartView,
    ModeSummaryField, ModeSummaryLabel, ModeSummaryRow, ModeSummaryScope, ModeTable,
    ModeValuesView, NetView, PartRow, PartView, PartsQuery, PortView, ProjectInfo, SettingView,
    SystemModeView, SystemModesRequest, SystemModesView, TreeNode, TreeNodeKind, ValueAnswer,
    ValueQuery,
};
pub use edit::{
    ApplyRequest, ApplyResult, ChangedFile, FileChangeKind, InitResult, PlaceBodyRequest, SetText,
    UnsetText,
};
pub use fune_model::design::PartField;
pub use fune_model::design::{
    AttributeDef, Case, CatalogField, CatalogItem, Distribution, Harness, KindDef, Maneuver,
    ManeuverCategory, ManeuverConstraints, ManeuverMethod, ManeuverTarget, ModeSegment, ModeWhen,
    Net, NetField, NetKind, Pose, Requirement, Source, SourceKind, StartFrom, StartRef,
    UncertaintySide,
};
pub use fune_model::design::{BudgetKind, SharedCatalogState, SharedCatalogStatus, StateAssign};
pub use fune_model::design::{ViewEdge, ViewLayout, ViewNode, ViewPortSide};
pub use fune_model::path::PortRef;
pub use fune_model::resolve::{Evidence, Origin};
pub use fune_model::value::{Provenance, Quantity, Quaternion, SettingValue, Value, Vector3};
pub use fune_model::{Diagnostic, Severity};
pub use fune_ops::{ModeCell, ModePosition, Op, ViewNodePos};
pub use fune_projection::MissingInput;
pub use history::{HistoryAction, HistoryEntryView, HistoryStepResult, HistoryView};
pub use sat_analysis::spec::{
    AnalysisArea, AnalysisInfo, AnalysisKind, Channel, Check, ConditionTopic, Series, System,
    Verdict,
};
pub use units::{
    DimensionInfo, FormatItem, FormatRequest, FormatResponse, ParseQuantityRequest, ParsedQuantity,
    UnitInfo, UnitMapping, UnitsCatalog, UnitsQuery,
};
pub use value::{ValueData, ValueView};

pub use crate::export::{ExportError, export_ts};
