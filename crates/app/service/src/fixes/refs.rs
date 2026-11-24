//! 別のもの（部品・カタログ品・通信の回線・マヌーバ）を指す欄の候補（`Fix::PickRef`）。選べるものはここで設計から絞り、
//! 受け取る側（CLI・Web）は並べるだけ。カタログ品の位置づけの綴りと種別の判定は `fune_model::design::launch` のものを使い、
//! ここに種別の表を持たない。

use super::{ANY, Design, MissingInput, Role, Target, pattern_matches, quote};
use fune_model::design::{LaunchInterface, LinkDirection};
use fune_model::path::{SettingPath, SettingScope};
use fune_projection::{Choice, Fix, RefKind};

/// 欄の場所（対象と表示パス）。
type Site<'a> = (&'a Target, &'a str);

/// 欄が指すものの絞り込み（部品の役割・カタログ品の種別。使わない方は空）。
#[derive(Default)]
struct Scope {
    roles: Vec<Role>,
    kinds: Vec<String>,
}

/// 選ぶ候補の組み立て。
fn pick(
    (target, path): Site<'_>,
    what: RefKind,
    scope: Scope,
    choices: Vec<Choice>,
    command: String,
    add_command: Option<String>,
) -> Fix {
    Fix::PickRef {
        target: target.clone(),
        path: path.to_owned(),
        what,
        roles: scope.roles,
        kinds: scope.kinds,
        choices,
        command,
        add_command,
    }
}

/// 部品を選ぶ候補。`parts` は選べる部品の ID（表示名は部品の `label`）。
pub(super) fn pick_part(
    design: &Design,
    (target, m): (&Target, &MissingInput),
    roles: &[Role],
    parts: Vec<String>,
    command: String,
) -> Fix {
    let choices = parts
        .into_iter()
        .map(|id| Choice {
            label: design
                .parts
                .get(id.as_str())
                .and_then(|p| p.label.clone())
                .unwrap_or_else(|| id.clone()),
            label_en: None,
            value: id,
        })
        .collect();
    let scope = Scope {
        roles: roles.to_vec(),
        kinds: Vec::new(),
    };
    let add = Some("fune part add <ID> --kind <kind>".to_owned());
    pick(
        (target, &m.path),
        RefKind::Part,
        scope,
        choices,
        command,
        add,
    )
}

/// 設定の欄がカタログ品を指すなら、その品を選ぶ候補。打ち上げ条件の全体（`conditions.launch`）が無いときは、
/// 打ち上げ機を選ぶ欄で代える。そうでなければ None。
pub(super) fn catalog_item(design: &Design, s: &SettingPath) -> Option<Fix> {
    let whole = pattern_matches(SettingScope::Conditions, &["launch"], s);
    if !whole && !pattern_matches(SettingScope::Conditions, &["launch", ANY], s) {
        return None;
    }
    let which: LaunchInterface = if whole {
        LaunchInterface::Launcher
    } else {
        s.segments.get(1)?.parse().ok()?
    };
    let kind = which.kind_id();
    let choices = design
        .catalog
        .values()
        .filter(|c| design.is_launch_kind_of(c.kind.as_str(), which))
        .map(|c| Choice {
            value: c.id.to_string(),
            label: c.label.clone().unwrap_or_else(|| c.id.to_string()),
            label_en: c.label_en.clone(),
        })
        .collect();
    let field = SettingPath::conditions(&["launch", which.as_str()]);
    let (path, target) = (field.to_string(), Target::from(field));
    let scope = Scope {
        roles: Vec::new(),
        kinds: vec![kind.to_owned()],
    };
    let command = format!("fune set {} <item>", quote(&path));
    let add = Some(format!("fune catalog add local/<ID> --kind {kind}"));
    Some(pick(
        (&target, &path),
        RefKind::CatalogItem,
        scope,
        choices,
        command,
        add,
    ))
}

/// `analysis.comm.data_downlink`: 下り（`direction = "down"`）の回線の名前から選ぶ候補。
pub(super) fn comm_downlink(
    design: &Design,
    site: (&Target, &MissingInput),
    s: &SettingPath,
) -> Option<Fix> {
    if !pattern_matches(SettingScope::Analysis, &["comm", "data_downlink"], s) {
        return None;
    }
    let choices = design
        .analysis
        .comm
        .iter()
        .flat_map(|c| c.links.iter())
        .filter(|(_, l)| l.direction == LinkDirection::Down)
        .map(|(name, _)| Choice {
            value: name.clone(),
            label: name.clone(),
            label_en: None,
        })
        .collect();
    let command = format!("fune set {} <link>", quote(&site.1.path));
    let add = "fune link add <ID> --direction down --part <part> --band <band>";
    let add = Some(add.to_owned());
    Some(pick(
        (site.0, &site.1.path),
        RefKind::CommLink,
        Scope::default(),
        choices,
        command,
        add,
    ))
}

/// `conditions.orbit.maneuver`: 日付つきの遷移（`[transfer]`）を持つマヌーバから選ぶ候補（巡航の軌道を取るマヌーバ）。
pub(super) fn orbit_maneuver(
    design: &Design,
    site: (&Target, &MissingInput),
    s: &SettingPath,
) -> Option<Fix> {
    if !pattern_matches(SettingScope::Conditions, &["orbit", "maneuver"], s) {
        return None;
    }
    let choices = design
        .ordered_maneuvers()
        .into_iter()
        .filter(|m| m.transfer.is_some())
        .map(|m| Choice {
            value: m.id.to_string(),
            label: m.label.clone().unwrap_or_else(|| m.id.to_string()),
            label_en: None,
        })
        .collect();
    let command = format!("fune set {} <maneuver>", quote(&site.1.path));
    let add = "fune maneuver add <ID> --category insertion --method impulsive --body <body> --depart <date> --arrive <date>";
    Some(pick(
        (site.0, &site.1.path),
        RefKind::Maneuver,
        Scope::default(),
        choices,
        command,
        Some(add.to_owned()),
    ))
}
