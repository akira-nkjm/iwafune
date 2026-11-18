//! ネットの分類（系統図の電源の系統と星形の中心）。**電源のネットの系統と、中心にするポートの選び方はここだけ。**
//! 部品の役割は解決済みの部品（`ResolvedPart.role`）から引き、中心の役割は `NetKind::hub_role`、戻り（GND）の判定は
//! `Net::is_return` を使う。Web はこの結果で色と線の向きを決めるだけで、役割名を見ない。

use crate::dto::{NetHub, PowerClass};
use fune_model::design::{Net, NetKind};
use fune_model::path::PortRef;
use fune_model::resolve::Resolved;
use fune_model::role::Role;

/// ポートを持つ部品の役割を引く関数（解決済みの部品から）。
pub(crate) fn roles(resolved: &Resolved) -> impl Fn(&PortRef) -> Option<Role> + '_ {
    |port| resolved.parts.get(port.part.as_str()).and_then(|p| p.role)
}

/// 電源のネットの系統。戻り（GND）→ 供給元が電池 → 太陽電池がつながる → レール電圧がある（安定化）→ 非安定化、の順に決める。
/// 電源以外は `None`。
pub(crate) fn power_class(
    net: &Net,
    members: &[PortRef],
    role_of: impl Fn(&PortRef) -> Option<Role>,
) -> Option<PowerClass> {
    if net.kind != NetKind::Power {
        return None;
    }
    let has = |role: Role| members.iter().any(|m| role_of(m) == Some(role));
    Some(if net.is_return() {
        PowerClass::Return
    } else if net.source.as_ref().and_then(&role_of) == Some(Role::Battery) {
        PowerClass::Battery
    } else if has(Role::SolarPanel) {
        PowerClass::Solar
    } else if net.rail.is_some() {
        PowerClass::Regulated
    } else {
        PowerClass::Unregulated
    })
}

/// 星形の中心。電源は供給元、データ・RF はマスターのポートが、このネットに属するポートの中にあればそれ。
/// 無ければ `NetKind::hub_role` の役割の部品のポート（属するポートの順で最初）を中心にする。役割から選んだ電源のネットは
/// 電力が中心の配電へ集まるので、線を各端から中心へ向ける（`collects`）。
pub(crate) fn hub(
    net: &Net,
    members: &[PortRef],
    role_of: impl Fn(&PortRef) -> Option<Role>,
) -> Option<NetHub> {
    let declared = match net.kind {
        NetKind::Power => net.source.as_ref(),
        NetKind::Data | NetKind::Rf => net.master.as_ref(),
    };
    if let Some(port) = declared.filter(|r| members.contains(r)) {
        return Some(NetHub {
            port: port.clone(),
            collects: false,
        });
    }
    let role = net.kind.hub_role();
    members
        .iter()
        .find(|m| role_of(m) == Some(role))
        .map(|port| NetHub {
            port: port.clone(),
            collects: net.kind == NetKind::Power,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::value::Quantity;

    /// 部品の ID から役割を引く表（名前は役割と同じにしてある）。
    fn role_of(p: &PortRef) -> Option<Role> {
        match p.part.as_str() {
            "bat" => Some(Role::Battery),
            "sp" => Some(Role::SolarPanel),
            "pcdu" | "pcdu2" => Some(Role::Pcdu),
            "obc" => Some(Role::Obc),
            _ => None,
        }
    }

    fn ports(xs: &[&str]) -> Vec<PortRef> {
        xs.iter().map(|p| PortRef::new(*p, "P")).collect()
    }

    #[test]
    fn power_class_follows_the_priority() {
        let mut n = Net::new("N", NetKind::Power);
        let class = |n: &Net, m: &[&str]| power_class(n, &ports(m), role_of);
        assert_eq!(class(&n, &["x"]), Some(PowerClass::Unregulated));
        n.rail = Some(Quantity::new(5.0, "V"));
        assert_eq!(class(&n, &["x"]), Some(PowerClass::Regulated));
        assert_eq!(class(&n, &["x", "sp"]), Some(PowerClass::Solar));
        n.source = Some(PortRef::new("bat", "P"));
        assert_eq!(class(&n, &["x", "sp"]), Some(PowerClass::Battery));
        n.id = "SA_GND".into();
        assert_eq!(class(&n, &["bat"]), Some(PowerClass::Return));
        let data = Net::new("D", NetKind::Data);
        assert_eq!(class(&data, &["obc"]), None);
    }

    #[test]
    fn hub_prefers_the_declared_port_then_the_role() {
        let mut n = Net::new("N", NetKind::Power);
        let members = ports(&["a", "pcdu2", "pcdu"]);
        let h = hub(&n, &members, role_of).unwrap();
        assert_eq!((h.port.part.as_str(), h.collects), ("pcdu2", true));
        n.source = Some(PortRef::new("a", "P"));
        let h = hub(&n, &members, role_of).unwrap();
        assert_eq!((h.port.part.as_str(), h.collects), ("a", false));
        // 属さないポートが書かれていれば、役割で選ぶ
        n.source = Some(PortRef::new("outside", "P"));
        assert_eq!(hub(&n, &members, role_of).unwrap().port.part, "pcdu2");
        let d = Net::new("D", NetKind::Data);
        let h = hub(&d, &ports(&["a", "obc"]), role_of).unwrap();
        assert_eq!((h.port.part.as_str(), h.collects), ("obc", false));
        assert!(hub(&d, &ports(&["a"]), role_of).is_none());
    }
}
