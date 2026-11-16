//! 電池の合成（容量・充電電力の上限の合計と、値の不一致の注意）の手計算。
use super::*;

fn battery() -> EnergyBalanceBattery {
    EnergyBalanceBattery {
        capacity: Qty::from_raw(36_000.0), // 10 Wh
        voltage: Qty::from_raw(8.0),
        charge_efficiency: Some(0.9),
        discharge_efficiency: Some(0.95),
        initial_soc: Some(1.0),
        dod_limit: 0.3,
        max_charge_power: None,
    }
}

#[test]
fn identical_batteries_sum_capacity_without_a_mismatch() {
    // 10 Wh = 36 kJ を 2 つ: 容量 72 kJ、電圧・効率・DoD は 1 つ分のまま、注意なし
    let total = EnergyBalanceBattery::combine(&[battery(), battery()]).unwrap();
    assert_eq!(total.battery.capacity.value(), 72_000.0);
    assert_eq!(total.battery.voltage.value(), 8.0);
    assert_eq!(total.battery.charge_eff(), 0.9);
    assert_eq!(total.battery.dod_limit, 0.3);
    assert!(total.mismatched.is_empty());
    assert!(total.mismatch_check().is_none());
}

#[test]
fn differing_voltage_is_named_in_a_warning() {
    let mut other = battery();
    other.voltage = Qty::from_raw(16.0);
    let total = EnergyBalanceBattery::combine(&[battery(), other]).unwrap();
    assert_eq!(total.mismatched, [BatteryField::Voltage]);
    let check = total.mismatch_check().unwrap();
    assert_eq!(check.key, "battery_mismatch");
    assert_eq!(check.verdict, Verdict::Warning);
    assert!(
        check.message.contains("Nominal voltage"),
        "{}",
        check.message
    );
    assert!(check.message.contains('8'), "{}", check.message);
    // 先頭の値を使う（並びを替えると 16 V になる）
    assert_eq!(total.battery.voltage.value(), 8.0);
}

#[test]
fn omitted_efficiency_equals_one_and_is_not_a_mismatch() {
    let mut a = battery();
    a.charge_efficiency = None;
    let mut b = battery();
    b.charge_efficiency = Some(1.0);
    assert!(
        EnergyBalanceBattery::combine(&[a, b])
            .unwrap()
            .mismatched
            .is_empty()
    );
}

#[test]
fn charge_power_limits_add_and_one_unlimited_pack_leaves_it_unlimited() {
    let (mut a, mut b) = (battery(), battery());
    a.max_charge_power = Some(Qty::from_raw(30.0));
    b.max_charge_power = Some(Qty::from_raw(20.0));
    let both = EnergyBalanceBattery::combine(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(both.battery.max_charge_power.unwrap().value(), 50.0);
    assert!(both.mismatched.is_empty());
    b.max_charge_power = None;
    let one = EnergyBalanceBattery::combine(&[a, b]).unwrap();
    assert!(one.battery.max_charge_power.is_none());
}

#[test]
fn differing_efficiency_prints_the_first_value_as_a_quantity() {
    let mut other = battery();
    other.charge_efficiency = Some(0.8);
    let check = EnergyBalanceBattery::combine(&[battery(), other])
        .unwrap()
        .mismatch_check()
        .unwrap();
    assert!(
        check.message.contains("Charge efficiency"),
        "{}",
        check.message
    );
}
