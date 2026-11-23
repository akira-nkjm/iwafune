//! 参照の索引: ある ID（部品・グループ・モード・ネット・カタログ品）を、どのエンティティのどのキーが持っているか。
//!
//! **参照の所在の知識はここだけ**。名前の変更と削除に伴う書き換え（`fune_store::write::refs`）、カタログ品の削除の拒否
//! （fune-ops）、カタログ品を使っている部品の一覧（`Design::catalog_part_users`。service の `CatalogItemView.users`）が、
//! どれもこの索引を引く。参照先が無いかの検査（`ref.*`）は向きが逆なので `check` にある。
//!
//! | 参照先 | 参照元（[`RefSite`]） |
//! |---|---|
//! | 部品 | モードの `[states]` と `[eclipse]` のキー・ネットの `source` / `master`・ハーネスの `from` / `to`・子部品の `mount.on`・配分のキー・`analysis/comm.toml` の `[link.*].part` / `antenna`・`analysis/thermal.toml` の `[nodes]` と `[surface]` と `[heat_node]` のキー・図の `[nodes]` のキー・マヌーバの `thruster` |
//! | グループ | 配分のキー・図の `[nodes]` のキー |
//! | モード | ケースの `mode`・`analysis/power.toml` の `[mode_share]` のキーと `[[sequence]]`・`[[sequences.<名前>.segments]]` の `mode` と `base_mode`・`analysis/aocs.toml` の `[law]` のキー・`analysis/thermal.toml` の `[case.*].mode`・マヌーバの `[constraints].mode`・フェーズの `modes` と `[law]` のキー |
//! | フェーズ | ケースの `phase` |
//! | ネット | 部品のポートの `net`・ハーネスの `nets`・図の `[nodes]` のキー |
//! | カタログ品 | 部品の `catalog`・`candidates`・ハーネスの `wire` |
//! | カタログ（`[[catalog]]` の名前。出所を外すのを拒むためだけ。書き換えはしない） | 部品の `catalog`・`candidates`・ハーネスの `wire` のうち `<名前>/…` のもの |
//! | 地上局 | `analysis/comm.toml` の `stations`・要求の `[verify].check` が名指す局（書き換えはしない。名前の変更と削除を拒むための暫定の見張り。#328） |
//! | ハーネス区間 | 参照元は無い（ほかのエンティティはハーネスの ID を持たない。名前の変更の索引を揃えるための空の索引） |
//! | 解析ケース | `analysis/common.toml` の `default_case`・要求の `[quantify].cases` |
//! | 出典 | 部品とカタログ品の属性の値の `from.source`（状態ごとのセルも）・カタログ品の `source` と `datasheet`・要求の `source` |
//! | マヌーバ | ほかのマヌーバの `after` |
//! | 通信の回線 | `analysis/comm.toml` の `data_downlink`・要求の `[verify].check` が名指す回線（`margin:<回線>@<局>`。書式は解析が持つので、局と同じく写し） |
//! | 種別（名前の変更と、プロジェクト独自の定義の削除の拒否に使う） | 部品とカタログ品の `kind`・種別の `extends` |
//! | 属性（同上） | 種別の `required` / `optional`・部品とカタログ品の属性の値 |

use super::{BudgetKind, Design};
use crate::enum_text::enum_text;
use crate::path::{PortRef, split_catalog_ref};
use serde::{Deserialize, Serialize};

mod records;

/// 参照される ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Referent<'a> {
    Part(&'a str),
    Group(&'a str),
    Mode(&'a str),
    /// フェーズの ID（`phases/<ID>.toml`）
    Phase(&'a str),
    Net(&'a str),
    /// カタログ品の参照（`"local/rw_10mNms"`）
    Catalog(&'a str),
    /// カタログの名前（`[[catalog]]` の `name`。その出所の品の参照すべて。外すのを拒むための索引）
    CatalogSource(&'a str),
    /// 地上局の ID（`conditions/ground/<ID>.toml`）
    Station(&'a str),
    /// 種別の ID（定義の削除を拒むためと、名前の変更で書き換えるための索引）
    Kind(&'a str),
    /// 属性の ID（同上）
    Attribute(&'a str),
    /// ハーネス区間の ID（参照元は無い）
    Harness(&'a str),
    /// 解析ケースの ID
    Case(&'a str),
    /// 出典の ID（`sources/<ID>.toml`）
    Source(&'a str),
    /// マヌーバの ID
    Maneuver(&'a str),
    /// 通信の回線の名前（`analysis/comm.toml` の `[link.<名前>]`）
    Link(&'a str),
}

enum_text! {
    /// ネットのポートの参照を持つキー（`nets/<ネット>.toml`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum NetEnd: "net end" {
        Source = "source",
        Master = "master",
    }
}

enum_text! {
    /// ハーネスの端のキー（`harness/<区間>.toml`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum HarnessEnd: "harness end" {
        From = "from",
        To = "to",
    }
}

/// 参照元 1 つ。どのファイルのどのキーかは store（`layout`）が決める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefSite {
    /// モードの表の行（`modes/<モード>.toml` の `[states]`・`[eclipse]` のキー）
    ModeRow { mode: String },
    /// ネットの供給元・マスター（値はポートの参照 `port`）
    NetPort {
        net: String,
        end: NetEnd,
        port: PortRef,
    },
    /// ハーネスの端（値はポートの参照 `port`）
    HarnessPort {
        harness: String,
        end: HarnessEnd,
        port: PortRef,
    },
    /// 子部品の取付先（`part` の `mount.on`）
    MountOn { part: String },
    /// 配分のキー（`budget/allocation.toml` の `[<種類>.alloc]`）
    Allocation { kind: BudgetKind },
    /// 回線の部品（`analysis/comm.toml` の `[link.<回線>].part`）
    CommLink { link: String },
    /// 回線の衛星側アンテナの部品（`analysis/comm.toml` の `[link.<回線>].antenna`）
    CommLinkAntenna { link: String },
    /// 熱の節点に入れた部品（`analysis/thermal.toml` の `[nodes]` の `<節点> = [...]`）
    ThermalNode { node: String },
    /// 外面の表面材を書いた部品（`analysis/thermal.toml` の `[surface.<部品>]` のキー）
    ThermalSurface,
    /// 発熱の場所を書いた部品（`analysis/thermal.toml` の `[heat_node]` のキー）
    ThermalHeatNode,
    /// 図の配置のノード（`view/<図>.toml` の `[nodes]` のキー）
    ViewNode { view: String },
    /// 解析ケースのモード（`cases/<ケース>.toml` の `mode`）
    CaseMode { case: String },
    /// 電源の時間割合のキー（`analysis/power.toml` の `[mode_share]`）
    PowerModeShare,
    /// 電源のモードの並びの区間（`[[sequence]]` の `index` 番目の `mode`）
    PowerSequence { index: usize },
    /// 電源の名前付きのモードの並びの区間（`[[sequences.<名前>.segments]]` の `index` 番目の `mode`）
    PowerNamedSequence { sequence: String, index: usize },
    /// 電源の基本のモード（`analysis/power.toml` の `base_mode`）
    PowerBaseMode,
    /// 姿勢則のキー（`analysis/aocs.toml` の `[law]`）
    AttitudeLaw,
    /// 熱の環境条件のモード（`analysis/thermal.toml` の `[case.<条件>].mode`）
    ThermalCaseMode { case: String },
    /// 部品のポートのネット（`ports.<ポート>.net`）
    PortNet { part: String, port: String },
    /// ハーネスが通すネット（`nets`）
    HarnessNet { harness: String },
    /// 部品の選択中のカタログ品（`catalog`）
    PartCatalog { part: String },
    /// 部品のトレード候補（`candidates`）
    PartCandidate { part: String },
    /// ハーネスの線材（`wire`）
    HarnessWire { harness: String },
    /// マヌーバのスラスタ（`maneuvers/<マヌーバ>.toml` の `thruster`）
    ManeuverThruster { maneuver: String },
    /// マヌーバの噴射するモード（`[constraints].mode`）
    ManeuverMode { maneuver: String },
    /// 解析ケースのフェーズ（`cases/<ケース>.toml` の `phase`）
    CasePhase { case: String },
    /// フェーズが使うモード（`phases/<フェーズ>.toml` の `modes`）
    PhaseMode { phase: String },
    /// フェーズの姿勢則のキー（`phases/<フェーズ>.toml` の `[law]`）
    PhaseLaw { phase: String },
    /// 通信の解析が使う地上局（`analysis/comm.toml` の `stations`）
    CommStation,
    /// 要求の判定のキーが名指す地上局（`requirements/<要求>.toml` の `[verify].check` の `@<局>`。キーが構造を持つまでの
    /// 暫定の見張りで、書き換えはしない。`Verification::check_names_station`）
    RequirementCheck { requirement: String },
    /// 部品の種別（`kind`）
    PartKind { part: String },
    /// カタログ品の種別（`kind`。`item` は `local/rw_10mNms` の形）
    CatalogKind { item: String },
    /// 種別の継承元（`extends`）
    KindExtends { kind: String },
    /// 種別が挙げる属性（`required`・`optional`）
    KindAttribute { kind: String },
    /// 部品が値を書いた属性（`[attr.<ID>]`）
    PartAttribute { part: String },
    /// カタログ品が値を書いた属性
    CatalogAttribute { item: String },
    /// 共通設定の既定のケース（`analysis/common.toml` の `default_case`）
    CommonDefaultCase,
    /// 要求が量化するケース（`requirements/<要求>.toml` の `[quantify].cases`）
    RequirementCase { requirement: String },
    /// 部品の属性の値の出典（`[attr.<属性>]` の `from.source`。状態ごとのセルの出典も）
    PartValueSource { part: String, attr: String },
    /// カタログ品の属性の値の出典（`item` は `local/rw_10mNms` の形）
    CatalogValueSource { item: String, attr: String },
    /// カタログ品の既定の出典（`source`）
    CatalogItemSource { item: String },
    /// カタログ品のデータシートの出典（`datasheet`）
    CatalogItemDatasheet { item: String },
    /// 要求の出典（`source`）
    RequirementSource { requirement: String },
    /// マヌーバの前（`after`）
    ManeuverAfter { maneuver: String },
    /// 通信の解析の下りの回線（`analysis/comm.toml` の `data_downlink`）
    CommDataDownlink,
    /// 要求の判定のキーが名指す回線（`[verify].check` の `margin:<回線>`・`pfd:<回線>`。`@<局>` は付いてもよい）。
    /// 解析のキーの書式の写し（`Verification::check_names_link`。キーが構造を持つまでの暫定。#328）。名前の変更では書き換える
    RequirementLinkCheck { requirement: String },
}

impl Design {
    /// `to` を参照している場所（参照元の種類の順、同じ種類の中は ID の順）。
    pub fn references(&self, to: Referent) -> Vec<RefSite> {
        match to {
            Referent::Part(id) => self.part_refs(id),
            Referent::Group(id) => self.group_refs(id),
            Referent::Mode(id) => self.mode_refs(id),
            Referent::Phase(id) => self.phase_refs(id),
            Referent::Net(id) => self.net_refs(id),
            Referent::Catalog(reference) => self.catalog_refs(|r| r == reference),
            Referent::CatalogSource(name) => {
                self.catalog_refs(|r| split_catalog_ref(r).is_some_and(|(s, _)| s == name))
            }
            Referent::Station(id) => self.station_refs(id),
            Referent::Kind(id) => self.kind_refs(id),
            Referent::Attribute(id) => self.attribute_refs(id),
            Referent::Harness(id) => self.harness_refs(id),
            Referent::Case(id) => self.case_refs(id),
            Referent::Source(id) => self.source_refs(id),
            Referent::Maneuver(id) => self.maneuver_refs(id),
            Referent::Link(id) => self.link_refs(id),
        }
    }

    fn part_refs(&self, id: &str) -> Vec<RefSite> {
        let mut out: Vec<RefSite> = self
            .modes
            .values()
            .filter(|m| m.states.contains_key(id) || m.eclipse.contains_key(id))
            .map(|m| RefSite::ModeRow {
                mode: m.id.to_string(),
            })
            .collect();
        for net in self.nets.values() {
            for (end, port) in [(NetEnd::Source, &net.source), (NetEnd::Master, &net.master)] {
                if let Some(port) = port.as_ref().filter(|p| p.part == id) {
                    out.push(RefSite::NetPort {
                        net: net.id.to_string(),
                        end,
                        port: port.clone(),
                    });
                }
            }
        }
        for h in self.harnesses.values() {
            for (end, port) in [(HarnessEnd::From, &h.from), (HarnessEnd::To, &h.to)] {
                if port.part == id {
                    out.push(RefSite::HarnessPort {
                        harness: h.id.to_string(),
                        end,
                        port: port.clone(),
                    });
                }
            }
        }
        out.extend(
            self.parts
                .values()
                .filter(|p| p.mount.as_ref().is_some_and(|m| m.on.as_str() == id))
                .map(|p| RefSite::MountOn {
                    part: p.id.to_string(),
                }),
        );
        out.extend(self.node_refs(id));
        if let Some(comm) = &self.analysis.comm {
            out.extend(
                comm.links
                    .iter()
                    .filter(|(_, l)| l.part == id)
                    .map(|(link, _)| RefSite::CommLink { link: link.clone() }),
            );
            out.extend(
                comm.links
                    .iter()
                    .filter(|(_, l)| l.antenna.as_deref() == Some(id))
                    .map(|(link, _)| RefSite::CommLinkAntenna { link: link.clone() }),
            );
        }
        if let Some(thermal) = &self.analysis.thermal {
            out.extend(
                thermal
                    .nodes
                    .iter()
                    .filter(|(_, parts)| parts.iter().any(|p| p == id))
                    .map(|(node, _)| RefSite::ThermalNode { node: node.clone() }),
            );
            if thermal.surface.contains_key(id) {
                out.push(RefSite::ThermalSurface);
            }
            if thermal.heat_node.contains_key(id) {
                out.push(RefSite::ThermalHeatNode);
            }
        }
        out.extend(
            self.maneuvers
                .values()
                .filter(|m| m.thruster.as_ref().is_some_and(|t| t.as_str() == id))
                .map(|m| RefSite::ManeuverThruster {
                    maneuver: m.id.to_string(),
                }),
        );
        out
    }

    fn group_refs(&self, id: &str) -> Vec<RefSite> {
        self.node_refs(id)
    }

    /// 部品とグループに共通の参照元（配分のキー・図のノード）。
    fn node_refs(&self, id: &str) -> Vec<RefSite> {
        let alloc = self
            .budget
            .allocation
            .iter()
            .filter(|(_, a)| a.alloc.contains_key(id))
            .map(|(kind, _)| RefSite::Allocation { kind });
        let views = self
            .views
            .values()
            .filter(|v| v.nodes.contains_key(id))
            .map(|v| RefSite::ViewNode { view: v.id.clone() });
        alloc.chain(views).collect()
    }

    fn mode_refs(&self, id: &str) -> Vec<RefSite> {
        let mut out: Vec<RefSite> = self
            .cases
            .values()
            .filter(|c| c.mode.as_ref().is_some_and(|m| m.as_str() == id))
            .map(|c| RefSite::CaseMode {
                case: c.id.to_string(),
            })
            .collect();
        let analysis = &self.analysis;
        if let Some(p) = &analysis.power {
            if p.mode_share.contains_key(id) {
                out.push(RefSite::PowerModeShare);
            }
            out.extend(
                p.sequence
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.mode == id)
                    .map(|(index, _)| RefSite::PowerSequence { index }),
            );
            for (name, s) in &p.sequences {
                out.extend(
                    s.segments
                        .iter()
                        .enumerate()
                        .filter(|(_, seg)| seg.mode == id)
                        .map(|(index, _)| RefSite::PowerNamedSequence {
                            sequence: name.clone(),
                            index,
                        }),
                );
            }
            if p.base_mode.as_deref() == Some(id) {
                out.push(RefSite::PowerBaseMode);
            }
        }
        if analysis
            .aocs
            .as_ref()
            .is_some_and(|a| a.law.contains_key(id))
        {
            out.push(RefSite::AttitudeLaw);
        }
        if let Some(t) = &analysis.thermal {
            out.extend(
                t.cases
                    .iter()
                    .filter(|(_, c)| c.mode.as_deref() == Some(id))
                    .map(|(case, _)| RefSite::ThermalCaseMode { case: case.clone() }),
            );
        }
        out.extend(
            self.maneuvers
                .values()
                .filter(|m| {
                    m.constraints
                        .mode
                        .as_ref()
                        .is_some_and(|x| x.as_str() == id)
                })
                .map(|m| RefSite::ManeuverMode {
                    maneuver: m.id.to_string(),
                }),
        );
        for p in self.phases.values() {
            if p.modes.iter().any(|m| m.as_str() == id) {
                out.push(RefSite::PhaseMode {
                    phase: p.id.to_string(),
                });
            }
            if p.law.contains_key(id) {
                out.push(RefSite::PhaseLaw {
                    phase: p.id.to_string(),
                });
            }
        }
        out
    }

    /// フェーズを指しているケース。
    fn phase_refs(&self, id: &str) -> Vec<RefSite> {
        self.cases
            .values()
            .filter(|c| c.phase.as_ref().is_some_and(|p| p.as_str() == id))
            .map(|c| RefSite::CasePhase {
                case: c.id.to_string(),
            })
            .collect()
    }

    fn net_refs(&self, id: &str) -> Vec<RefSite> {
        let mut out = Vec::new();
        for part in self.parts.values() {
            for (port, binding) in &part.ports {
                if binding.net.as_ref().is_some_and(|n| n.as_str() == id) {
                    out.push(RefSite::PortNet {
                        part: part.id.to_string(),
                        port: port.clone(),
                    });
                }
            }
        }
        out.extend(
            self.harnesses
                .values()
                .filter(|h| h.nets.iter().any(|n| n.as_str() == id))
                .map(|h| RefSite::HarnessNet {
                    harness: h.id.to_string(),
                }),
        );
        // 図の配置のネットのノード（見た目だけ。部品・グループのノードと同じキー）
        out.extend(
            self.views
                .values()
                .filter(|v| v.nodes.contains_key(id))
                .map(|v| RefSite::ViewNode { view: v.id.clone() }),
        );
        out
    }

    /// カタログ品の参照のうち `hit` に当たるもの。部品ごとに（ID 順）選択中 → 候補、そのあとハーネス。
    /// カタログの名前（`Referent::CatalogSource`）は品が読めていない（共有カタログを取得していない）ときも参照の文字列で探す。
    fn catalog_refs(&self, hit: impl Fn(&str) -> bool) -> Vec<RefSite> {
        let mut out = Vec::new();
        for p in self.parts.values() {
            let part = || p.id.to_string();
            if p.catalog.as_deref().is_some_and(&hit) {
                out.push(RefSite::PartCatalog { part: part() });
            }
            if p.candidates.iter().any(|c| hit(c)) {
                out.push(RefSite::PartCandidate { part: part() });
            }
        }
        out.extend(
            self.harnesses
                .values()
                .filter(|h| h.wire.as_deref().is_some_and(&hit))
                .map(|h| RefSite::HarnessWire {
                    harness: h.id.to_string(),
                }),
        );
        out
    }

    /// 地上局を使っているもの（通信の解析の `stations`（空なら全局を使うので、どの局も参照されない）→ 判定のキーが名指す要求）。
    fn station_refs(&self, id: &str) -> Vec<RefSite> {
        let listed = self
            .analysis
            .comm
            .as_ref()
            .is_some_and(|c| c.stations.iter().any(|s| s == id));
        let mut out = Vec::new();
        if listed {
            out.push(RefSite::CommStation);
        }
        out.extend(
            self.requirements
                .values()
                .filter(|r| r.verify.check_names_station(id))
                .map(|r| RefSite::RequirementCheck {
                    requirement: r.id.to_string(),
                }),
        );
        out
    }

    /// 種別を使っているもの: 部品 → カタログ品 → 継承している種別の順。
    fn kind_refs(&self, id: &str) -> Vec<RefSite> {
        let parts = self
            .parts
            .values()
            .filter(|p| p.kind.as_ref().is_some_and(|k| k.as_str() == id))
            .map(|p| RefSite::PartKind {
                part: p.id.to_string(),
            });
        let items = self
            .catalog
            .iter()
            .filter(|(_, c)| c.kind.as_str() == id)
            .map(|(reference, _)| RefSite::CatalogKind {
                item: reference.to_string(),
            });
        let kinds = self
            .kinds
            .values()
            .filter(|k| k.extends.as_ref().is_some_and(|e| e.as_str() == id))
            .map(|k| RefSite::KindExtends {
                kind: k.id.to_string(),
            });
        parts.chain(items).chain(kinds).collect()
    }

    /// 属性を使っているもの: 種別 → 部品 → カタログ品の順。種別はプロジェクト独自のものだけ
    /// （組み込みの種別が挙げるのは組み込みの属性なので、プロジェクト独自の属性の削除には関わらない）。
    fn attribute_refs(&self, id: &str) -> Vec<RefSite> {
        let kinds = self
            .kinds
            .values()
            .filter(|k| {
                k.required
                    .iter()
                    .chain(&k.optional)
                    .any(|a| a.as_str() == id)
            })
            .map(|k| RefSite::KindAttribute {
                kind: k.id.to_string(),
            });
        let parts = self
            .parts
            .values()
            .filter(|p| p.attr.contains_key(id))
            .map(|p| RefSite::PartAttribute {
                part: p.id.to_string(),
            });
        let items = self
            .catalog
            .iter()
            .filter(|(_, c)| c.attr.contains_key(id))
            .map(|(reference, _)| RefSite::CatalogAttribute {
                item: reference.to_string(),
            });
        kinds.chain(parts).chain(items).collect()
    }
}
