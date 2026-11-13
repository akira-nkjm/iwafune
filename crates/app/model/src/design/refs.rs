//! 参照の索引: ある ID（部品・グループ・モード・ネット・カタログ品）を、どのエンティティのどのキーが持っているか。
//!
//! **参照の所在の知識はここだけ**。名前の変更と削除に伴う書き換え（`fune_store::write::refs`）、カタログ品の削除の拒否
//! （fune-ops）、カタログ品を使っている部品の一覧（`Design::catalog_part_users`。service の `CatalogItemView.users`）が、
//! どれもこの索引を引く。参照先が無いかの検査（`ref.*`）は向きが逆なので `check` にある。
//!
//! | 参照先 | 参照元（[`RefSite`]） |
//! |---|---|
//! | 部品 | モードの `[states]` のキー・ネットの `source` / `master`・ハーネスの `from` / `to`・子部品の `mount.on`・配分のキー・`analysis/comm.toml` の `[link.*].part`・`analysis/thermal.toml` の `[nodes]`・図の `[nodes]` のキー |
//! | グループ | 配分のキー・図の `[nodes]` のキー |
//! | モード | ケースの `mode`・`analysis/power.toml` の `[mode_share]` のキーと `[[sequence]]` の `mode` と `base_mode`・`analysis/aocs.toml` の `[law]` のキー・`analysis/thermal.toml` の `[case.*].mode` |
//! | ネット | 部品のポートの `net`・ハーネスの `nets` |
//! | カタログ品 | 部品の `catalog`・`candidates`・ハーネスの `wire` |

use super::{BudgetKind, Design};
use crate::enum_text::enum_text;
use crate::path::PortRef;
use serde::{Deserialize, Serialize};

/// 参照される ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Referent<'a> {
    Part(&'a str),
    Group(&'a str),
    Mode(&'a str),
    Net(&'a str),
    /// カタログ品の参照（`"local/rw_10mNms"`）
    Catalog(&'a str),
}

enum_text! {
    /// ネットのポートの参照を持つキー（`nets/<ネット>.toml`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum NetEnd: "ネットの端" {
        Source = "source",
        Master = "master",
    }
}

enum_text! {
    /// ハーネスの端のキー（`harness/<区間>.toml`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum HarnessEnd: "ハーネスの端" {
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
}

impl Design {
    /// `to` を参照している場所（参照元の種類の順、同じ種類の中は ID の順）。
    pub fn references(&self, to: Referent) -> Vec<RefSite> {
        match to {
            Referent::Part(id) => self.part_refs(id),
            Referent::Group(id) => self.group_refs(id),
            Referent::Mode(id) => self.mode_refs(id),
            Referent::Net(id) => self.net_refs(id),
            Referent::Catalog(reference) => self.catalog_refs(reference),
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
        out
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
        out
    }

    /// 部品ごとに（ID 順）選択中 → 候補、そのあとハーネス。
    fn catalog_refs(&self, reference: &str) -> Vec<RefSite> {
        let mut out = Vec::new();
        for p in self.parts.values() {
            let part = || p.id.to_string();
            if p.catalog.as_deref() == Some(reference) {
                out.push(RefSite::PartCatalog { part: part() });
            }
            if p.candidates.iter().any(|c| c == reference) {
                out.push(RefSite::PartCandidate { part: part() });
            }
        }
        out.extend(
            self.harnesses
                .values()
                .filter(|h| h.wire.as_deref() == Some(reference))
                .map(|h| RefSite::HarnessWire {
                    harness: h.id.to_string(),
                }),
        );
        out
    }
}
