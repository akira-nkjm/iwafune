//! 出力の時系列と判定。

use super::trace::Trace;
use super::{RwUnloadingInput, RwUnloadingOutput};
use crate::attitude::disturbance::{DISTURBANCE_KINDS, DisturbanceTorques};
use crate::spec::quantity_text;
use crate::spec::{Channel, Check, Series, SpanFallback, Verdict, short_span_check};
use sat_astro::math::Vec3;

pub(super) fn series(input: &RwUnloadingInput, tr: &Trace) -> Series {
    let torque = |key: &str, label: &str, f: &dyn Fn(&DisturbanceTorques) -> f64| {
        let v = tr.torques.iter().map(f).collect();
        Channel::new(&format!("torque:{key}"), label, "Torque", v)
    };
    let momentum = |key: &str, label: &str, f: &dyn Fn(&Vec3) -> f64| {
        let v = tr.h_body.iter().map(f).collect();
        Channel::new(&format!("momentum:{key}"), label, "AngularMomentum", v)
    };
    let mut channels: Vec<Channel> = DISTURBANCE_KINDS
        .iter()
        .enumerate()
        .map(|(i, (key, label))| torque(key, label, &|d| d.by_kind()[i].norm()))
        .collect();
    channels.push(torque("total", "擾乱トルクの合計", &|d| {
        d.total().norm()
    }));
    channels.push(momentum("x", "角運動量 x（機体）", &|h| h.x));
    channels.push(momentum("y", "角運動量 y（機体）", &|h| h.y));
    channels.push(momentum("z", "角運動量 z（機体）", &|h| h.z));
    channels.push(momentum("norm", "角運動量の大きさ", &|h| h.norm()));
    for (w, values) in input.wheels.iter().zip(&tr.wheel_h) {
        let label = format!("{} の角運動量", w.id);
        let key = format!("wheel:{}", w.id);
        channels.push(Channel::new(
            &key,
            &label,
            "AngularMomentum",
            values.clone(),
        ));
    }
    Series {
        t: tr.t.clone(),
        channels,
    }
}

pub(super) fn verdicts(input: &RwUnloadingInput, o: &RwUnloadingOutput) -> Vec<Check> {
    let mut checks = Vec::new();
    let zero_interval = o.unload_interval.is_some_and(|i| i.value() <= 0.0);
    checks.push(Check::new(
        "momentum_capacity",
        "角運動量の容量",
        Verdict::from_ok(!zero_interval),
        if zero_interval {
            "周期成分だけで使える角運動量を超えます".to_owned()
        } else {
            format!(
                "周期成分は容量の {}",
                quantity_text(
                    o.cyclic_amplitude.value() / o.capacity.value(),
                    sat_units::typed::Fraction::KIND
                )
            )
        },
    ));
    if let Some(m) = o.torque_margin {
        checks.push(Check::new(
            "torque_margin",
            "トルクの余裕",
            Verdict::from_ok(m >= 1.0),
            format!(
                "最大トルク / 必要トルク = {}",
                quantity_text(m, sat_units::typed::Fraction::KIND)
            ),
        ));
    }
    if let Some(req) = input.required_unload_interval {
        let ok = o.unload_interval.is_none_or(|i| i >= req);
        checks.push(Check::new(
            "unload_interval",
            "アンロード間隔の要求",
            Verdict::from_ok(ok),
            format!("要求 {} 以上", quantity_text(req.value(), "Time")),
        ));
    }
    checks.extend(short_span_check(
        o.complete_orbits,
        SpanFallback::SecularExtrapolated,
    ));
    checks
}
