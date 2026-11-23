//! 解決の計算（値の重ね合わせと SI 換算）。

use super::{
    Cell, Evidence, Origin, PoseSi, Resolved, ResolvedMount, ResolvedPart, ResolvedPort,
    ResolvedValue, SiScalar, SiValue,
};
use crate::design::{
    AttributeDef, CatalogItem, Design, EntityKey, Part, Pose, SourceKind, SourceMap,
};
use crate::id::Id;
use crate::value::units::to_si;
use crate::value::{Authored, Provenance, Scalar, Value, ValueError};
use std::collections::{BTreeMap, BTreeSet};

/// Design を解決する（純粋関数）。
pub fn resolve(design: &Design) -> Resolved {
    let parts: BTreeMap<_, _> = design
        .parts
        .iter()
        .map(|(id, part)| (id.clone(), resolve_part(design, part)))
        .collect();
    let configurations = design
        .configurations
        .values()
        .map(|c| {
            let id = c.id.as_str();
            (id.to_owned(), resolve_configuration(design, &parts, id))
        })
        .collect();
    Resolved {
        parts,
        configurations,
    }
}

/// 形態 `configuration` で解決した設計。姿勢が違うのは取付だけなので、基準の解決済みの部品から取付だけを
/// その形態の姿勢（`Design::pose_in`）で作り直す（値の重ね合わせは基準のまま）。
fn resolve_configuration(
    design: &Design,
    base: &BTreeMap<Id<Part>, ResolvedPart>,
    configuration: &str,
) -> Resolved {
    let parts = base
        .iter()
        .map(|(id, resolved)| {
            let mut resolved = resolved.clone();
            if let Some(part) = design.parts.get(id) {
                resolved.mount = resolve_mount(design, part, Some(configuration));
            }
            (id.clone(), resolved)
        })
        .collect();
    Resolved {
        parts,
        configurations: BTreeMap::new(),
    }
}

fn resolve_part(design: &Design, part: &Part) -> ResolvedPart {
    let catalog = part.catalog.as_ref().and_then(|c| design.catalog.get(c));
    let kind = design.part_kind_id(part).map(str::to_string);
    let mut ids: BTreeSet<String> = BTreeSet::new();
    if let Some(kind) = &kind {
        for k in design.kind_chain(kind) {
            ids.extend(k.required.iter().chain(&k.optional).map(|a| a.to_string()));
        }
    }
    if let Some(item) = catalog {
        ids.extend(item.attr.keys().map(|a| a.to_string()));
    }
    ids.extend(part.attr.keys().map(|a| a.to_string()));
    let attr: BTreeMap<String, ResolvedValue> = ids
        .into_iter()
        .map(|id| {
            let value = resolve_attr(design, part, catalog, &id);
            (id, value)
        })
        .collect();
    let evidence = attr
        .values()
        .filter(|v| v.origin != Origin::Unset)
        .map(|v| v.evidence)
        .min()
        .unwrap_or(Evidence::Unsourced);
    ResolvedPart {
        id: part.id.to_string(),
        label: part
            .label
            .clone()
            .or_else(|| catalog.and_then(|c| c.label.clone())),
        parent: part.parent.clone(),
        system: design.system_of(part.id.as_str()),
        catalog: part.catalog.clone(),
        role: kind.as_deref().and_then(|k| design.kind_role(k)),
        states: kind
            .as_deref()
            .map(|k| design.kind_states(k).to_vec())
            .unwrap_or_default(),
        kind,
        count: part.count,
        redundancy: part.redundancy,
        maturity: part.maturity.or_else(|| catalog.and_then(|c| c.maturity)),
        attr,
        mount: resolve_mount(design, part, None),
        ports: resolve_ports(design, part, catalog),
        evidence,
    }
}

/// 1 つの属性を解決する（上書き → カタログ → 既定値）。
fn resolve_attr(
    design: &Design,
    part: &Part,
    catalog: Option<&CatalogItem>,
    id: &str,
) -> ResolvedValue {
    let over = part.attr.get(id);
    let base = catalog.and_then(|c| c.attr.get(id).map(|b| (c, b)));
    let authored = match (over, base) {
        (Some(o), Some((_, b))) => merge_by_state(b, o),
        (Some(o), None) => o.clone(),
        (None, Some((item, b))) => return catalog_value(design, item, id, b),
        (None, None) => return unset_or_default(design.attribute(id)),
    };
    let loc = locate_attr(&design.source_map, EntityKey::Part(part.id.to_string()), id);
    authored_value(design, id, authored, Origin::Override, None, loc)
}

/// カタログ品に書かれた値を解決する（属性 ID → 値）。部品がその品を選んで上書きしていないときと同じ値
/// （出所は `catalog`、出典の無い値の根拠は品の `source` から）。カタログの画面が部品の値と同じ形で見せるのに使う。
pub fn resolve_catalog_item(
    design: &Design,
    item: &CatalogItem,
) -> BTreeMap<String, ResolvedValue> {
    item.attr
        .iter()
        .map(|(id, b)| (id.to_string(), catalog_value(design, item, id.as_str(), b)))
        .collect()
}

/// カタログ品の値 1 つ（部品から見たときも、品そのものを見るときも同じ）。
fn catalog_value(
    design: &Design,
    item: &CatalogItem,
    id: &str,
    authored: &Authored<Value>,
) -> ResolvedValue {
    let loc = locate_attr(&design.source_map, EntityKey::Catalog(item.id.clone()), id);
    authored_value(
        design,
        id,
        authored.clone(),
        Origin::Catalog,
        item.source.as_deref(),
        loc,
    )
}

/// 書かれた値 → 解決済みの値（SI・根拠・次元・表示単位）。`default_source` は出典の無い値に当てる出典。
fn authored_value(
    design: &Design,
    id: &str,
    authored: Authored<Value>,
    origin: Origin,
    default_source: Option<&str>,
    loc: Option<crate::design::Loc>,
) -> ResolvedValue {
    let def = design.attribute(id);
    let value = match si_value(&authored.value) {
        Ok(v) => Cell::Set(v),
        Err(_) => Cell::Unset,
    };
    ResolvedValue {
        value,
        origin,
        evidence: value_evidence(design, &authored, default_source),
        dimension: def.and_then(|d| d.dimension.clone()),
        display_unit: def
            .map(|d| d.display_unit().to_string())
            .unwrap_or_default(),
        from: authored.from.clone(),
        authored: Some(authored.value),
        loc,
    }
}

fn locate_attr(map: &SourceMap, entity: EntityKey, id: &str) -> Option<crate::design::Loc> {
    map.locate(&entity, &["attr", id])
}

/// 状態ごとの値は状態ごとに上書きする（書かれていない状態はカタログの値のまま）。
fn merge_by_state(base: &Authored<Value>, over: &Authored<Value>) -> Authored<Value> {
    match (&base.value, &over.value) {
        (Value::ByState(b), Value::ByState(o)) => {
            let mut merged = b.clone();
            merged.extend(o.iter().map(|(k, v)| (k.clone(), v.clone())));
            Authored {
                value: Value::ByState(merged),
                from: over.from.clone(),
            }
        }
        _ => over.clone(),
    }
}

fn unset_or_default(def: Option<&AttributeDef>) -> ResolvedValue {
    let (value, origin) = match def.and_then(|d| d.default) {
        Some(v) => (
            Cell::Set(SiValue::Scalar(SiScalar {
                value: v,
                min: None,
                max: None,
            })),
            Origin::KindDefault,
        ),
        None => (Cell::Unset, Origin::Unset),
    };
    // 既定値は仮定なので、根拠は placeholder（画面で「仮置き」と分かるように）。
    let evidence = if origin == Origin::KindDefault {
        Evidence::Placeholder
    } else {
        Evidence::Unsourced
    };
    ResolvedValue {
        value,
        origin,
        evidence,
        dimension: def.and_then(|d| d.dimension.clone()),
        display_unit: def
            .map(|d| d.display_unit().to_string())
            .unwrap_or_default(),
        authored: None,
        from: None,
        loc: None,
    }
}

/// 書かれた値を SI に直す（単位の解析は sat-units）。
pub fn si_value(value: &Value) -> Result<SiValue, ValueError> {
    Ok(match value {
        Value::Scalar(s) => SiValue::Scalar(si_scalar(s)?),
        Value::ByState(m) => SiValue::ByState(
            m.iter()
                .map(|(k, v)| Ok((k.clone(), si_scalar(&v.value)?)))
                .collect::<Result<_, ValueError>>()?,
        ),
        Value::Vec3(v) => {
            let unit = v.unit.as_deref().unwrap_or("");
            SiValue::Vec3([to_si(v.x, unit)?, to_si(v.y, unit)?, to_si(v.z, unit)?])
        }
        Value::Quat(q) => SiValue::Quat([q.w, q.x, q.y, q.z]),
        Value::Tensor3(t) => {
            let mut m = [[0.0; 3]; 3];
            for (row, src) in m.iter_mut().zip(&t.m) {
                for (cell, v) in row.iter_mut().zip(src) {
                    *cell = to_si(*v, &t.unit)?;
                }
            }
            SiValue::Tensor3(m)
        }
        Value::Enum(s) => SiValue::Enum(s.clone()),
        Value::Bool(b) => SiValue::Bool(*b),
        Value::Text(s) => SiValue::Text(s.clone()),
        Value::Range(r) => SiValue::Range {
            min: to_si(r.min, &r.unit)?,
            max: to_si(r.max, &r.unit)?,
        },
        Value::Curve(c) => SiValue::Curve {
            at: c
                .at
                .iter()
                .map(|v| to_si(*v, &c.at_unit))
                .collect::<Result<_, _>>()?,
            values: c
                .values
                .iter()
                .map(|v| to_si(*v, &c.unit))
                .collect::<Result<_, _>>()?,
        },
        Value::Table(t) => SiValue::Table(t.clone()),
    })
}

fn si_scalar(s: &Scalar) -> Result<SiScalar, ValueError> {
    Ok(SiScalar {
        value: to_si(s.value, &s.unit)?,
        min: s.min.map(|m| to_si(m, &s.unit)).transpose()?,
        max: s.max.map(|m| to_si(m, &s.unit)).transpose()?,
    })
}

/// 値の根拠（状態ごとの値はその最弱値）。
fn value_evidence(design: &Design, value: &Authored<Value>, default: Option<&str>) -> Evidence {
    let own = evidence_of(design, value.from.as_ref(), default);
    match &value.value {
        Value::ByState(m) => m
            .values()
            .map(|c| evidence_of(design, c.from.as_ref().or(value.from.as_ref()), default))
            .min()
            .unwrap_or(own),
        _ => own,
    }
}

/// 出典の種類 → 根拠。出典が無い・未登録なら Unsourced。
pub fn evidence_of(design: &Design, from: Option<&Provenance>, default: Option<&str>) -> Evidence {
    let source = from.and_then(|f| f.source.as_deref()).or(default);
    match source.and_then(|s| design.sources.get(s)).map(|s| s.kind) {
        Some(SourceKind::Placeholder) => Evidence::Placeholder,
        Some(SourceKind::Judgement) => Evidence::Judgement,
        Some(SourceKind::Analysis) => Evidence::Analysis,
        Some(SourceKind::Datasheet) => Evidence::Datasheet,
        Some(SourceKind::Measured) => Evidence::Measured,
        None => Evidence::Unsourced,
    }
}

/// 書かれた姿勢を SI に直す（位置は m、回転は書かれたまま）。読めない値は 0 / 恒等とみなす（check が報告する）。
pub fn pose_si(pose: Option<&Pose>) -> PoseSi {
    let Some(pose) = pose else {
        return PoseSi::IDENTITY;
    };
    let position = pose.position.as_ref().map_or([0.0; 3], |p| {
        let unit = crate::design::position_unit(p);
        [
            to_si(p.x, unit).unwrap_or(0.0),
            to_si(p.y, unit).unwrap_or(0.0),
            to_si(p.z, unit).unwrap_or(0.0),
        ]
    });
    let orientation = pose
        .orientation
        .as_ref()
        .map_or([1.0, 0.0, 0.0, 0.0], |q| [q.w, q.x, q.y, q.z]);
    PoseSi {
        position,
        orientation,
    }
}

/// 取付 1 段。姿勢は形態 `configuration`（`None` は基準）で有効な書かれた姿勢（`Design::pose_in`。**取付の姿勢を決めるのはここだけ**）。
fn resolve_mount(
    design: &Design,
    part: &Part,
    configuration: Option<&str>,
) -> Option<ResolvedMount> {
    let mount = part.mount.as_ref()?;
    let parent = design.parts.get(mount.on.as_str());
    let point_pose = mount
        .point
        .as_ref()
        .and_then(|p| parent.and_then(|parent| parent.mount_points.get(p)))
        .map_or(PoseSi::IDENTITY, |p| pose_si(Some(p)));
    Some(ResolvedMount {
        on: mount.on.to_string(),
        point: mount.point.clone(),
        point_pose,
        pose: pose_si(design.pose_in(part, configuration)),
    })
}

fn resolve_ports(
    design: &Design,
    part: &Part,
    catalog: Option<&CatalogItem>,
) -> BTreeMap<String, ResolvedPort> {
    let mut templates = BTreeMap::new();
    if let Some(kind) = design.part_kind_id(part) {
        for k in design.kind_chain(kind).into_iter().rev() {
            templates.extend(k.ports.iter().map(|(p, kind)| (p.clone(), *kind)));
        }
    }
    if let Some(item) = catalog {
        templates.extend(item.ports.iter().map(|(p, kind)| (p.clone(), *kind)));
    }
    let mut ports: BTreeMap<String, ResolvedPort> = templates
        .iter()
        .map(|(p, kind)| {
            (
                p.clone(),
                ResolvedPort {
                    net: None,
                    kind: Some(*kind),
                },
            )
        })
        .collect();
    for (name, binding) in &part.ports {
        let net = binding.net.as_ref().map(|n| n.to_string());
        let kind = net
            .as_deref()
            .and_then(|n| design.nets.get(n))
            .map(|n| n.kind)
            .or_else(|| templates.get(name).copied());
        ports.insert(name.clone(), ResolvedPort { net, kind });
    }
    ports
}
