//! 不足入力の直し方の候補（`fune_projection::Fix`）を決める。**候補の決め方と、直しのコマンドの文はここだけ**
//! （`fune analyze` の表示も GUI の行の操作も、これが返した候補を使う）。
//!
//! 射影（`fune-projection`）は対象（`Target`）を名指しして不足を返すだけで、設計の中の候補の部品などは知らない。
//! ここで設計（`Design`・`Resolved`）を見て、対象の種類ごとに操作の種類と絞り込みの条件を決める。
//!
//! | 対象 | 候補 |
//! |---|---|
//! | 部品の属性・フィールド | `SetValue`（部品がカタログ品を選んでいれば品の側の書き先も） |
//! | 設定 | `SetValue`（次元は `setting_dimension`）。回線の部品・アンテナの欄は `PickPart` |
//! | 役割のどれかの部品 | その役割の部品が無ければ `AddPart` と `AssignPart`、有って値が無ければ `SetValueOnPart` |
//! | マヌーバの `thruster` | `PickPart` |
//! | モード・マヌーバ・地上局・回線・ケースの一覧の空 | `AddListItem` |
//!
//! どの候補も既存の Op になる（新しい編集の道は作らない）。

use fune_model::design::setting_dims::{ANY, pattern_matches, setting_dimension};
use fune_model::design::{Design, Maneuver};
use fune_model::path::{ParamPath, SettingPath, SettingScope};
use fune_model::resolve::{Origin, Resolved};
use fune_model::role::Role;
use fune_model::value::units::unit_dimension;
use fune_model::{Id, Target};
use fune_ops::{Op, cli::quote, to_cli};
use fune_projection::{CatalogFix, Fix, ListKind, MissingInput};

/// 置き換え欄（値）。コマンドの中では `"<value>"` と引用して書く。
const VALUE: &str = "\"<value>\"";

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
        Target::Maneuver { id, field } if field.as_deref() == Some("thruster") => {
            maneuver_thruster(design, (target, m), id)
        }
        _ => Vec::new(),
    }
}

/// 値を書く先（部品の属性・フィールド・設定）に値を入れる候補。
fn set_value(m: &MissingInput, target: ParamPath, dimension: Option<String>) -> Fix {
    Fix::SetValue {
        path: m.path.clone(),
        command: format!("fune set {} {VALUE}", quote(&m.path)),
        target,
        dimension,
        catalog: None,
    }
}

/// 属性の次元名（定義の `dimension`、無ければ `units` の先頭の単位の次元）。
fn attr_dimension(design: &Design, attr: &str) -> Option<String> {
    let def = design.attribute(attr)?;
    def.dimension
        .clone()
        .or_else(|| def.units.first().and_then(|u| unit_dimension(u).ok()))
}

/// 部品の属性に値を入れる候補。部品がカタログ品を選んでいれば、品の側へ書く代わりの書き先も付ける。
fn set_attr(design: &Design, m: &MissingInput, part: &str, attr: &str, state: Option<&str>) -> Fix {
    let path = ParamPath::Attr {
        part: part.to_owned(),
        attr: attr.to_owned(),
        state: state.map(str::to_owned),
    };
    let mut fix = set_value(m, path, attr_dimension(design, attr));
    let item = design.parts.get(part).and_then(|p| p.catalog.as_ref());
    if let (Fix::SetValue { catalog, .. }, Some(item)) = (&mut fix, item) {
        let inside = state.map_or_else(|| attr.to_owned(), |s| format!("{attr}.{s}"));
        let item = item.to_string();
        *catalog = Some(CatalogFix {
            command: format!(
                "fune catalog set {} {} {VALUE}",
                quote(&item),
                quote(&inside)
            ),
            item,
            path: inside,
        });
    }
    fix
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
        return vec![pick_part(site, roles, parts, command)];
    }
    if pattern_matches(SettingScope::Analysis, &["comm", "link"], s) {
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
    let dimension = setting_dimension(s).map(str::to_owned);
    vec![set_value(m, ParamPath::Setting(s.clone()), dimension)]
}

fn add_item(list: ListKind, command: &str) -> Fix {
    Fix::AddListItem {
        list,
        command: command.to_owned(),
    }
}

fn pick_part(
    (target, m): (&Target, &MissingInput),
    roles: &[Role],
    parts: Vec<String>,
    command: String,
) -> Fix {
    Fix::PickPart {
        target: target.clone(),
        path: m.path.clone(),
        roles: roles.to_vec(),
        parts,
        command,
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
    vec![pick_part(site, &roles, parts, command)]
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
