//! 系統図の電源の系統と星形の中心は service が決める（Web は役割名を見ない）。デモの値は、Web が役割名で決めていたときと同じ。
use fune_service::dto::{NetView, PowerClass};
use fune_service::{Workspace, query};
use std::path::Path;

fn net(nets: &[NetView], id: &str) -> NetView {
    nets.iter().find(|n| n.id == id).unwrap().clone()
}

#[test]
fn demo_nets_carry_the_class_and_the_hub() {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let nets = query::design_view(&ws).unwrap().nets;
    let class = |id| net(&nets, id).power_class;
    assert_eq!(class("BAT_BUS"), Some(PowerClass::Battery));
    assert_eq!(class("SA_BUS"), Some(PowerClass::Solar));
    assert_eq!(class("P5V"), Some(PowerClass::Regulated));
    assert_eq!(class("CAN_BUS"), None);
    // 供給元・マスターが書かれていればそのポート。線は中心から各端へ
    let hub = net(&nets, "P12V").hub.unwrap();
    assert_eq!(
        (hub.port.to_string().as_str(), hub.collects),
        ("PCDU.OUT3", false)
    );
    let hub = net(&nets, "CAN_BUS").hub.unwrap();
    assert_eq!(
        (hub.port.to_string().as_str(), hub.collects),
        ("OBC.CAN1", false)
    );
    // 供給元の無い太陽電池の母線は、役割（PCDU）が中心で、線は各端から中心へ
    let hub = net(&nets, "SA_BUS").hub.unwrap();
    assert_eq!(
        (hub.port.to_string().as_str(), hub.collects),
        ("PCDU.SA", true)
    );
}
