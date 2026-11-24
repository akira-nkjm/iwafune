//! 不足入力の直し方の候補（`fune_projection::Fix`）を決める。**候補の決め方と、直しのコマンドの文はここだけ**
//! （`fune analyze` の表示も GUI の行の操作も、これが返した候補を使う）。
//!
//! 射影（`fune-projection`）は対象（`Target`）を名指しして不足を返すだけで、設計の中の候補の部品などは知らない。
//! ここで設計（`Design`・`Resolved`）を見て、対象の種類ごとに操作の種類と絞り込みの条件を決める。
//!
//! | 対象 | 候補 |
//! |---|---|
//! | 部品の属性・フィールド | `SetValue`（部品がカタログ品を選んでいれば品の側の書き先も。量には入力の例、列挙・真偽には選べる値） |
//! | 設定 | `SetValue`（次元は `setting_dimension`。`conditions.launch` の欄が列挙の属性と同じ名前なら、選べる値は属性定義の語彙）。回線の部品・アンテナの欄は `PickRef`（部品）、打ち上げ機・ディスペンサーは `PickRef`（カタログ品）、`data_downlink` は `PickRef`（回線）、軌道の `maneuver` は `PickRef`（日付つきの遷移を持つマヌーバ）。軌道の `source` は候補なし（対象外の理由） |
//! | 役割のどれかの部品 | その役割の部品が無ければ `AddPart` と `AssignPart`、有って値が無ければ `SetValueOnPart` |
//! | マヌーバの `thruster` | `PickRef`（部品） |
//! | モード・マヌーバ・地上局・回線・ケースの一覧の空 | `AddListItem` |
//!
//! どの候補も既存の Op になる（新しい編集の道は作らない）。

use fune_model::design::setting_dims::{ANY, pattern_matches, setting_dimension};
use fune_model::design::{Design, Maneuver};
use fune_model::path::{ParamPath, SettingPath, SettingScope};
use fune_model::resolve::{Origin, Resolved};
use fune_model::role::Role;
use fune_model::{Id, Target};
use fune_ops::{Op, cli::quote, to_cli};
use fune_projection::{Fix, ListKind, MissingInput};

mod refs;
mod value;

use refs::pick_part;
use value::{VALUE, attr_dimension, example_of, set_attr, set_value};

/// 不足入力の一覧に、設計を見て決めた直し方の候補を付ける。サービスが不足入力を返す所はすべてこれを通す。
pub fn complete(
    design: &Design,
    resolved: &Resolved,
    missing: Vec<MissingInput>,
) -> Vec<MissingInput> {
    missing
        .into_iter()
        .map(|mut m| {
            m.fixes = fixes_for(design, resolved, &m);
            m
        })
        .collect()
}

/// 不足入力 1 件の直し方の候補。
pub fn fixes_for(design: &Design, resolved: &Resolved, m: &MissingInput) -> Vec<Fix> {
    let Some(target) = &m.target else {
        return Vec::new();
    };
    match target {
        Target::Param(ParamPath::Attr { part, attr, state }) => {
            vec![set_attr(design, m, part, attr, state.as_deref())]
        }
        Target::Param(p @ ParamPath::Field { .. }) => vec![set_value(m, p.clone(), None)],
        Target::Param(ParamPath::Setting(s)) => setting_fixes(design, (target, m), s),
        Target::AnyPart { roles, attr } => any_part(design, resolved, roles, attr.as_deref()),
        Target::Modes => vec![add_item(ListKind::Mode, "fune mode add <ID>")],
        Target::Maneuvers => vec![add_item(
            ListKind::Maneuver,
            "fune maneuver add <ID> --category <category> --method <method>",
        )],
        Target::Case { id, field: None } => {
            vec![add_item(
                ListKind::Case,
                &format!("fune case set {}", quote(id)),
            )]
        }
        Target::Phase { id, field } if field.as_deref() == Some("configuration") => {
            refs::phase_configuration(design, (target, m), id)
                .into_iter()
                .collect()
        }
        Target::Maneuver { id, field } if field.as_deref() == Some("thruster") => {
            maneuver_thruster(design, (target, m), id)
        }
        _ => Vec::new(),
    }
}

/// 設定が足りないときの候補（部品を指す欄・一覧の空・量の値）。
fn setting_fixes(design: &Design, site: (&Target, &MissingInput), s: &SettingPath) -> Vec<Fix> {
    let comm =
        |last: &str| pattern_matches(SettingScope::Analysis, &["comm", "link", ANY, last], s);
    let roles: &[Role] = if comm("part") {
        &[Role::Transceiver]
    } else if comm("antenna") {
        &[Role::Antenna]
    } else {
        &[]
    };
    let m = site.1;
    if !roles.is_empty() {
        let parts = parts_of_roles(design, roles);
        let command = format!("fune set {} <part>", quote(&m.path));
        return vec![pick_part(design, site, roles, parts, command)];
    }
    if pattern_matches(SettingScope::Analysis, &["comm"], s)
        || pattern_matches(SettingScope::Analysis, &["comm", "link"], s)
    {
        let command = "fune link add <ID> --direction <down|up> --part <part> --band <band>";
        return vec![add_item(ListKind::CommLink, command)];
    }
    if pattern_matches(SettingScope::Conditions, &["ground"], s) {
        return vec![add_item(ListKind::GroundStation, "fune ground add <ID>")];
    }
    if pattern_matches(SettingScope::Conditions, &["ground", ANY], s) {
        let id = s.segments.get(1).map_or("<ID>", String::as_str);
        let command = format!("fune ground add {}", quote(id));
        return vec![add_item(ListKind::GroundStation, &command)];
    }
    // 軌道の出どころ（`source`）は書かなくてよい欄で、足りない入力として挙がるのは「この解析には対象外」の理由だけ（直す候補は無い）
    if pattern_matches(SettingScope::Conditions, &["orbit", "source"], s) {
        return Vec::new();
    }
    if let Some(fix) = refs::catalog_item(design, s)
        .or_else(|| refs::comm_downlink(design, site, s))
        .or_else(|| refs::launch_configuration(design, site, s))
        .or_else(|| refs::orbit_maneuver(design, site, s))
    {
        return vec![fix];
    }
    if let Some(fix) = value::launch_enum(design, m, s) {
        return vec![fix];
    }
    let dimension = setting_dimension(s).map(str::to_owned);
    vec![set_value(m, ParamPath::Setting(s.clone()), dimension)]
}

fn add_item(list: ListKind, command: &str) -> Fix {
    Fix::AddListItem {
        list,
        command: command.to_owned(),
    }
}

/// マヌーバの `thruster` を選ぶ候補。コマンドはマヌーバを丸ごと書く `fune maneuver set`（今の値のまま、スラスタだけ置き換え欄）。
fn maneuver_thruster(design: &Design, site: (&Target, &MissingInput), id: &str) -> Vec<Fix> {
    let parts: Vec<String> = design
        .parts
        .values()
        .filter(|p| design.is_maneuver_thruster(p))
        .map(|p| p.id.to_string())
        .collect();
    let command = design
        .maneuvers
        .get(id)
        .map(set_thruster_command)
        .unwrap_or_else(|| {
            format!(
                "fune maneuver set {} --category <category> --method <method> --thruster <part>",
                quote(id)
            )
        });
    let roles = [Role::PropulsionThruster, Role::Thruster];
    vec![pick_part(design, site, &roles, parts, command)]
}

/// 今のマヌーバのスラスタを置き換え欄にした `fune maneuver set` のコマンド（`fune_ops::to_cli` が書く）。
fn set_thruster_command(maneuver: &Maneuver) -> String {
    let mut maneuver = maneuver.clone();
    maneuver.thruster = Some(Id::new("<part>"));
    to_cli(&Op::SetManeuver { maneuver }).join(" ")
}

/// 役割のどれかを持つ種別（継承を含む）の ID。
fn kinds_of_roles(design: &Design, roles: &[Role]) -> Vec<String> {
    design
        .all_kinds()
        .keys()
        .filter(|k| design.kind_role(k).is_some_and(|r| roles.contains(&r)))
        .map(|k| (*k).to_owned())
        .collect()
}

/// 部品のうち、役割のどれかを持つものの ID（役割が空ならすべて）。
fn parts_of_roles(design: &Design, roles: &[Role]) -> Vec<String> {
    design
        .parts
        .values()
        .filter(|p| {
            roles.is_empty()
                || design
                    .part_kind_id(p)
                    .and_then(|k| design.kind_role(k))
                    .is_some_and(|r| roles.contains(&r))
        })
        .map(|p| p.id.to_string())
        .collect()
}

/// 「どれかの部品」の不足の候補（役割と属性の組で分かれる）。
fn any_part(design: &Design, resolved: &Resolved, roles: &[Role], attr: Option<&str>) -> Vec<Fix> {
    let have = parts_of_roles(design, roles);
    match attr {
        Some(attr) if !have.is_empty() => {
            let lacking: Vec<String> = have
                .into_iter()
                .filter(|id| !has_value(resolved, id, attr))
                .collect();
            vec![Fix::SetValueOnPart {
                attr: attr.to_owned(),
                dimension: attr_dimension(design, attr),
                example: example_of(attr_dimension(design, attr).as_deref()),
                choices: value::choices_of_attr(design, attr),
                parts: lacking,
                command: format!("fune set <part>.{} {VALUE}", quote(attr)),
            }]
        }
        // 役割の部品が既にあるときの不足は、部品ごとの不足（属性の値）として別に出るので、ここでは足さない
        _ if roles.is_empty() || !have.is_empty() => Vec::new(),
        _ => role_missing(design, roles),
    }
}

/// 解決済みの部品の属性に値があるか（書かれたか、カタログ・既定・導出で入っているか）。
fn has_value(resolved: &Resolved, part: &str, attr: &str) -> bool {
    resolved
        .parts
        .get(part)
        .and_then(|p| p.attr.get(attr))
        .is_some_and(|v| v.origin != Origin::Unset)
}

/// 役割の部品が 1 つも無いときの候補: 部品を足す・既にある部品を役割のものに替える。
fn role_missing(design: &Design, roles: &[Role]) -> Vec<Fix> {
    let kinds = kinds_of_roles(design, roles);
    let kind_text = match kinds.as_slice() {
        [one] => quote(one),
        _ => "<kind>".to_owned(),
    };
    let catalog_items: Vec<String> = design
        .catalog
        .values()
        .filter(|c| kinds.iter().any(|k| k == c.kind.as_str()))
        .map(|c| c.id.to_string())
        .collect();
    let with_role = parts_of_roles(design, roles);
    let others: Vec<String> = design
        .parts
        .keys()
        .map(ToString::to_string)
        .filter(|id| !with_role.contains(id))
        .collect();
    let mut out = vec![Fix::AddPart {
        roles: roles.to_vec(),
        kinds: kinds.clone(),
        catalog_items,
        command: format!("fune part add <ID> --kind {kind_text}"),
    }];
    if !others.is_empty() {
        out.push(Fix::AssignPart {
            roles: roles.to_vec(),
            kinds,
            parts: others,
            command: format!("fune set <part>.kind {kind_text}"),
        });
    }
    out
}
