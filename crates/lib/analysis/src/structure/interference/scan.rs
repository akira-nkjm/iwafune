//! 全組の走査（`structure.interference`）。取付の連鎖・同じ ID を除いた組ごとに、重なり・縁の接触・離れている組へ振り分ける。

use super::edge_contact::{EdgeContact, classify};
use super::obb::{OrientedBox, separation};
use super::{InterferenceBox, InterferenceClearance, InterferencePair, on_mount_chain};
use crate::structure::acknowledged::{Acknowledgement, find};
use sat_units::typed::{Length, Qty};

/// 全組の走査の結果。
pub(super) struct Scan {
    pub judged: usize,
    pub related: usize,
    pub overlaps: Vec<InterferencePair>,
    pub edge_contacts: Vec<EdgeContact>,
    pub clearance: Option<InterferenceClearance>,
}

/// 取付の連鎖の上にある（片方がもう片方の祖先の）組か。
fn in_mount_chain(a: &InterferenceBox, b: &InterferenceBox) -> bool {
    on_mount_chain(&a.id, &a.mount_ancestors, &b.id, &b.mount_ancestors)
}

/// 全組を調べる（組の順は ID の昇順で固定。軸は `a` から `b` へ）。
pub(super) fn scan_pairs(
    boxes: &[&InterferenceBox],
    tolerance: f64,
    acknowledged: &[Acknowledgement],
) -> Scan {
    let placed: Vec<OrientedBox> = boxes
        .iter()
        .map(|b| OrientedBox::from_size(b.position, b.orientation, b.size))
        .collect();
    let mut scan = Scan {
        judged: 0,
        related: 0,
        overlaps: Vec::new(),
        edge_contacts: Vec::new(),
        clearance: None,
    };
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (a, b) = (boxes[i], boxes[j]);
            // 同じ ID の箱は同じ部品として比べない
            if a.id == b.id {
                continue;
            }
            if in_mount_chain(a, b) {
                scan.related += 1;
                continue;
            }
            scan.judged += 1;
            scan.record(((a, &placed[i]), (b, &placed[j])), tolerance, acknowledged);
        }
    }
    scan.overlaps.sort_by(|x, y| {
        by_depth(
            x.depth,
            y.depth,
            (&x.first, &x.second),
            (&y.first, &y.second),
        )
    });
    scan.edge_contacts.sort_by(|x, y| {
        by_depth(
            x.depth,
            y.depth,
            (&x.first, &x.second),
            (&y.first, &y.second),
        )
    });
    scan
}

/// 深い順、同じ深さなら組の ID の昇順。
fn by_depth(
    x: Qty<Length>,
    y: Qty<Length>,
    xs: (&String, &String),
    ys: (&String, &String),
) -> std::cmp::Ordering {
    y.value().total_cmp(&x.value()).then_with(|| xs.cmp(&ys))
}

impl Scan {
    /// 判定する組 1 つを、重なり・縁の接触・離れている組のどれかに振り分ける。
    fn record(
        &mut self,
        ((a, pa), (b, pb)): (
            (&InterferenceBox, &OrientedBox),
            (&InterferenceBox, &OrientedBox),
        ),
        tolerance: f64,
        acknowledged: &[Acknowledgement],
    ) {
        let s = separation(pa, pb);
        if s.depth() <= tolerance {
            let gap = s.distance.max(0.0);
            if self.clearance.as_ref().is_none_or(|c| gap < c.gap.value()) {
                self.clearance = Some(InterferenceClearance {
                    first: a.id.clone(),
                    second: b.id.clone(),
                    gap: Qty::from_raw(gap),
                });
            }
            return;
        }
        // 縁に沿った薄い切れ端だけの重なりは、干渉にせず縁の接触として別に残す
        if let Some(basis) = classify(pa, pb, tolerance) {
            self.edge_contacts.push(EdgeContact {
                first: a.id.clone(),
                second: b.id.clone(),
                depth: Qty::from_raw(s.depth()),
                thickness: Qty::from_raw(basis.thickness),
            });
            return;
        }
        let ack = find(acknowledged, &a.id, &b.id, false);
        self.overlaps.push(InterferencePair {
            first: a.id.clone(),
            second: b.id.clone(),
            depth: Qty::from_raw(s.depth()),
            separation_axis: s.axis,
            acknowledged: ack.is_some(),
            reason: ack.map(|a| a.reason.clone()),
            acknowledged_configuration: ack.and_then(|a| a.configuration.clone()),
        });
    }
}
