//! # sat-checkkey — 解析の判定のキーの書式（唯一の正本）
//!
//! 解析の判定（`Check`）は文字列のキーを持ち、要求の `verify.check` がそのキーを指す。キーには設計のエンティティの名前
//! （通信の回線・地上局・部品・熱の節点）が入るものがある（`margin:<回線>@<局>`・`heater:<部品>` など）。
//!
//! **キーの組み立て（解析）と、キーから名前を読み出す・書き換える（モデルの参照の索引。`fune_model::design::refs`）が、
//! 同じ書式の表 [`FAMILIES`] を使う**ので、書式を直す場所はここだけ。sat-analysis と fune-model は互いに依存できないため、
//! どちらからも見える葉の crate に置く。
//!
//! 要求の `verify.check` は判定のキーの接頭辞でもよい（`margin`・`margin:<回線>`・`range_low:<節点>`）ので、読み出しは
//! 途中までのキーも読む。
//!
//! | 族 | 形 | 名指すもの |
//! |---|---|---|
//! | `margin`・`pfd` | `<族>:<回線>@<局>` | 通信の回線・地上局 |
//! | `link_closes` | `link_closes:<局>` | 地上局 |
//! | `intrusion` | `intrusion:<センサの部品>` | 部品 |
//! | `interface` | `interface:<取付先の部品>` | 部品（`LV` はロケットで部品ではない） |
//! | `heater` | `heater:<ヒータの部品>` | 部品 |
//! | `panel`・`lateral`・`axial` | `<族>:<パネルの部品>` | 部品 |
//! | `overlap` | `overlap:<部品>:<部品>` | 箱が重なっている 2 つの部品（ID の昇順） |
//! | `obstruction` | `obstruction:<担い手の部品>:<部品>` | 円錐（噴流・視野）を持つ部品と、その円錐に入る部品 |
//! | `range_low`・`range_high` | `<族>:<節点>:<範囲の種類>` | 熱の節点（`analysis/thermal.toml` の `[nodes]` のキー。部品の ID ではない） |

/// キーに名前が入るエンティティの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// 部品（`parts/` の ID）
    Part,
    /// 通信の回線（`analysis/comm.toml` の `[link.<名前>]`）
    Link,
    /// 地上局（`conditions/ground/<ID>.toml`）
    Station,
    /// 熱の節点（`analysis/thermal.toml` の `[nodes]` のキー）
    ThermalNode,
}

/// 族の名前（キーの先頭）。
pub mod family {
    pub const MARGIN: &str = "margin";
    pub const PFD: &str = "pfd";
    pub const LINK_CLOSES: &str = "link_closes";
    pub const INTRUSION: &str = "intrusion";
    pub const INTERFACE: &str = "interface";
    pub const HEATER: &str = "heater";
    pub const PANEL: &str = "panel";
    pub const LATERAL: &str = "lateral";
    pub const AXIAL: &str = "axial";
    pub const OVERLAP: &str = "overlap";
    pub const OBSTRUCTION: &str = "obstruction";
    pub const RANGE_LOW: &str = "range_low";
    pub const RANGE_HIGH: &str = "range_high";
}

/// キーの欄 1 つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// エンティティの名前
    Entity(EntityKind),
    /// 名前ではない欄（範囲の種類など）
    Plain,
}

/// 族の書式: 族の名前と、続く欄（区切りの文字、欄）の並び。
struct Family {
    name: &'static str,
    slots: &'static [(char, Slot)],
}

use EntityKind::{Link, Part, Station, ThermalNode};
use Slot::{Entity, Plain};

/// 書式の表。**キーの書式のただ 1 つの表**（組み立て・読み出し・書き換えのすべてがこれを使う）。
const FAMILIES: &[Family] = &[
    Family {
        name: family::MARGIN,
        slots: &[(':', Entity(Link)), ('@', Entity(Station))],
    },
    Family {
        name: family::PFD,
        slots: &[(':', Entity(Link)), ('@', Entity(Station))],
    },
    Family {
        name: family::LINK_CLOSES,
        slots: &[(':', Entity(Station))],
    },
    Family {
        name: family::INTRUSION,
        slots: &[(':', Entity(Part))],
    },
    Family {
        name: family::INTERFACE,
        slots: &[(':', Entity(Part))],
    },
    Family {
        name: family::HEATER,
        slots: &[(':', Entity(Part))],
    },
    Family {
        name: family::PANEL,
        slots: &[(':', Entity(Part))],
    },
    Family {
        name: family::LATERAL,
        slots: &[(':', Entity(Part))],
    },
    Family {
        name: family::AXIAL,
        slots: &[(':', Entity(Part))],
    },
    Family {
        name: family::OVERLAP,
        slots: &[(':', Entity(Part)), (':', Entity(Part))],
    },
    Family {
        name: family::OBSTRUCTION,
        slots: &[(':', Entity(Part)), (':', Entity(Part))],
    },
    Family {
        name: family::RANGE_LOW,
        slots: &[(':', Entity(ThermalNode)), (':', Plain)],
    },
    Family {
        name: family::RANGE_HIGH,
        slots: &[(':', Entity(ThermalNode)), (':', Plain)],
    },
];

/// `name` が書式の表にある族の名前か（族の名前を持つ側の検査用）。
pub fn is_family(name: &str) -> bool {
    FAMILIES.iter().any(|f| f.name == name)
}

/// キーの組み立て（解析が使う）。欄を書式の表の順に並べる。
fn build(family: &str, values: &[&str]) -> String {
    let Some(f) = FAMILIES.iter().find(|f| f.name == family) else {
        // 表に無い族は組み立てない（コードの誤り。キーは族だけにする）
        return family.to_owned();
    };
    let mut out = family.to_owned();
    for ((sep, _), value) in f.slots.iter().zip(values) {
        out.push(*sep);
        out.push_str(value);
    }
    out
}

/// 通信の回線の余裕の判定のキー `margin:<回線>@<局>`。
pub fn link_margin(link: &str, station: &str) -> String {
    build(family::MARGIN, &[link, station])
}

/// 通信の回線の電力束密度の判定のキー `pfd:<回線>@<局>`。
pub fn link_pfd(link: &str, station: &str) -> String {
    build(family::PFD, &[link, station])
}

/// 局の回線が閉じないことの判定のキー `link_closes:<局>`。
pub fn link_closes(station: &str) -> String {
    build(family::LINK_CLOSES, &[station])
}

/// センサの禁止角への侵入の判定のキー `intrusion:<センサの部品>`。
pub fn sensor_intrusion(part: &str) -> String {
    build(family::INTRUSION, &[part])
}

/// 取付部の荷重の判定のキー `interface:<取付先>`。
pub fn mount_interface(mount: &str) -> String {
    build(family::INTERFACE, &[mount])
}

/// ヒータのデューティの判定のキー `heater:<ヒータの部品>`。
pub fn heater(part: &str) -> String {
    build(family::HEATER, &[part])
}

/// 部品の箱の重なりの判定のキー `overlap:<部品>:<部品>`。
pub fn part_overlap(first: &str, second: &str) -> String {
    build(family::OVERLAP, &[first, second])
}

/// 円錐（噴流・視野）を持つ部品の円錐に別の部品が入っていることの判定のキー `obstruction:<担い手の部品>:<部品>`。
pub fn cone_obstruction(carrier: &str, part: &str) -> String {
    build(family::OBSTRUCTION, &[carrier, part])
}

/// 固有振動数の判定のキー（`family` は [`family::PANEL`]・[`family::LATERAL`]・[`family::AXIAL`]）。
pub fn modal_frequency(family: &str, part: &str) -> String {
    build(family, &[part])
}

/// 温度範囲の判定のキー（`family` は [`family::RANGE_LOW`]・[`family::RANGE_HIGH`]。`kind` は範囲の種類）。
pub fn thermal_range(family: &str, node: &str, kind: &str) -> String {
    build(family, &[node, kind])
}

/// 読んだキーの 1 欄。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Field<'a> {
    sep: char,
    value: &'a str,
    slot: Slot,
}

/// 読んだキー。
struct Parsed<'a> {
    family: &'a str,
    parts: Vec<Field<'a>>,
}

/// キー（途中までのキーも）を族と欄に分ける。表に無い族なら `None`。
fn parse(key: &str) -> Option<Parsed<'_>> {
    let (name, mut rest) = key.split_once(':').map_or((key, ""), |(n, r)| (n, r));
    let family = FAMILIES.iter().find(|f| f.name == name)?;
    let has_rest = key.len() > name.len();
    let mut parts = Vec::new();
    if has_rest {
        for (i, (sep, slot)) in family.slots.iter().enumerate() {
            // 最初の欄の区切り（`:`）は族の分け目で読んである
            if i > 0 {
                rest = rest.strip_prefix(*sep)?;
            }
            let next = family.slots.get(i + 1).map(|(s, _)| *s);
            let (value, tail) = match next.and_then(|n| rest.find(n)) {
                Some(at) => rest.split_at(at),
                None => (rest, ""),
            };
            parts.push(Field {
                sep: *sep,
                value,
                slot: *slot,
            });
            rest = tail;
            if rest.is_empty() {
                break;
            }
        }
    }
    Some(Parsed {
        family: family.name,
        parts,
    })
}

/// キー `key` が名指している `kind` のエンティティの名前（なければ空）。
pub fn names(key: &str, kind: EntityKind) -> Vec<&str> {
    parse(key)
        .map(|p| {
            p.parts
                .into_iter()
                .filter(|part| part.slot == Entity(kind))
                .map(|part| part.value)
                .collect()
        })
        .unwrap_or_default()
}

/// `kind` のエンティティ `from` を名指すキーを、`to` を名指すものに替えたキー（名指していなければ `None`）。
/// 名前が欄の一部に当たるだけのとき（`from` = `s` は `margin:s_down@…` に当たらない）は替えない。
pub fn renamed(key: &str, kind: EntityKind, from: &str, to: &str) -> Option<String> {
    let parsed = parse(key)?;
    let hit = parsed
        .parts
        .iter()
        .any(|p| p.slot == Entity(kind) && p.value == from);
    hit.then(|| {
        let mut out = parsed.family.to_owned();
        for p in &parsed.parts {
            out.push(p.sep);
            let rewrite = p.slot == Entity(kind) && p.value == from;
            out.push_str(if rewrite { to } else { p.value });
        }
        out
    })
}

#[cfg(test)]
mod tests;
