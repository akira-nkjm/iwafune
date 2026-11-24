//! 出力の時系列と判定。

use super::trace::Trace;
use super::{RwUnloadingInput, RwUnloadingOutput, TORQUE_MARGIN_WARNING_BAND};
use crate::aocs::disturbance::{DisturbanceKind, DisturbanceTorques};
use crate::spec::quantity_text;
use crate::spec::{
    Channel, ChannelKind, Check, Series, Verdict, ratio_margin, ratio_margin_verdict,
};
use sat_astro::math::Vec3;
use sat_units::typed::{AngularMomentum, Fraction, Time, TorqueKind};

pub(super) fn series(input: &RwUnloadingInput, kinds: &[DisturbanceKind], tr: &Trace) -> Series {
    let torque = |key: &str, kind: ChannelKind, f: &dyn Fn(&DisturbanceTorques) -> f64| {
        let v = tr.torques.iter().map(f).collect();
        Channel::of::<TorqueKind>(format!("torque:{key}"), kind, v)
    };
    let momentum = |key: &str, kind: ChannelKind, f: &dyn Fn(&Vec3) -> f64| {
        let v = tr.h_body.iter().map(f).collect();
        Channel::of::<AngularMomentum>(format!("momentum:{key}"), kind, v)
    };
    let mut channels: Vec<Channel> = kinds
        .iter()
        .map(|&k| torque(k.key(), k.channel(), &|d| d.of(k).norm()))
        .collect();
    channels.push(torque("total", ChannelKind::TotalDisturbanceTorque, &|d| {
        d.total().norm()
    }));
    channels.push(momentum("x", ChannelKind::MomentumX, &|h| h.x));
    channels.push(momentum("y", ChannelKind::MomentumY, &|h| h.y));
    channels.push(momentum("z", ChannelKind::MomentumZ, &|h| h.z));
    channels.push(momentum("norm", ChannelKind::MomentumNorm, &|h| h.norm()));
    for (w, values) in input.wheels.iter().zip(&tr.wheel_h) {
        channels.push(
            Channel::of::<AngularMomentum>(
                format!("wheel:{}", w.id),
                ChannelKind::WheelMomentum,
                values.clone(),
            )
            .about(&w.id),
        );
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
        "Momentum capacity",
        Verdict::from_ok(!zero_interval),
        if zero_interval {
            "The periodic component alone exceeds the usable momentum".to_owned()
        } else {
            format!(
                "Periodic component is {} of capacity",
                quantity_text::<Fraction>(o.cyclic_amplitude.value() / o.capacity.value())
            )
        },
    ));
    if let Some(m) = o.min_torque_margin {
        checks.push(
            Check::new(
                "torque_margin",
                "Torque margin",
                ratio_margin_verdict(Some(m), TORQUE_MARGIN_WARNING_BAND),
                format!(
                    "Margin (max torque / required torque − 1) {}",
                    quantity_text::<Fraction>(m.value())
                ),
            )
            .with_margin(m.value()),
        );
    }
    if let Some(req) = input.required_unload_interval {
        let ok = o.unload_interval.is_none_or(|i| i >= req);
        checks.push(
            Check::new(
                "unload_interval",
                "Unload interval requirement",
                Verdict::from_ok(ok),
                format!("Required at least {}", quantity_text::<Time>(req.value())),
            )
            .with_ratio_margin(
                o.unload_interval
                    .and_then(|i| ratio_margin(i.value(), req.value())),
            ),
        );
    }
    checks.extend(
        input
            .spacecraft
            .environment_check(input.timeline.orbit().central()),
    );
    checks
}
