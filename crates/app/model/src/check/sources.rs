//! 出典の検査: 出典の欄の形（日付・SHA-256）、値とカタログ品（`source`・`datasheet`）からの出典の参照、
//! 書き込めないカタログ（共有カタログ）の品が公開できない出典を使っていないか（docs/model.md §7.8）。

use super::Ctx;
use crate::design::EntityKey;
use crate::design::source::is_location;
use crate::path::{AttrPath, ParamPath};
use crate::target::Target;
use crate::value::{Authored, Value, is_calendar_date};

/// 出典のすべての検査。
pub(super) fn check_sources(ctx: &mut Ctx) {
    check_source_fields(ctx);
    let uses = source_uses(ctx);
    check_refs(ctx, &uses);
    check_restricted(ctx, &uses);
}

/// SHA-256 の 16 進数の桁数。
const SHA256_HEX_LEN: usize = 64;

/// 出典の欄の形（`sha256` は 64 桁の 16 進数、`date`・`retrieved` は `YYYY-MM-DD`）。
fn check_source_fields(ctx: &mut Ctx) {
    let design = ctx.design;
    for source in design.sources.values() {
        let id = source.id.as_str();
        let entity = EntityKey::Source(id.to_owned());
        let target = || Some(Target::Source { id: id.to_owned() });
        if let Some(hash) = &source.sha256
            && !(hash.len() == SHA256_HEX_LEN && hash.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            ctx.error(
                "source.sha256",
                &entity,
                &["sha256"],
                target(),
                format!("sha256 は 64 桁の 16 進数にします（`{hash}`）"),
            );
        }
        for (key, date) in [("date", &source.date), ("retrieved", &source.retrieved)] {
            if let Some(date) = date
                && !is_calendar_date(date)
            {
                ctx.error(
                    "source.date",
                    &entity,
                    &[key],
                    target(),
                    format!("{key} は `2026-10-15` の形の日付にします（`{date}`）"),
                );
            }
        }
    }
}

/// 出典を参照している所 1 つ。
struct SourceUse<'a> {
    entity: EntityKey,
    /// 位置を引くキー（`["attr", "mass"]`・`["datasheet"]`）
    keys: Vec<String>,
    target: Target,
    source: &'a str,
    /// カタログ品の参照なら、その品の ID（`local/rw_10mNms`）
    item: Option<&'a str>,
    /// カタログ品の `datasheet` からの参照か
    datasheet: bool,
}

/// 部品とカタログ品の値の `from.source`、カタログ品の `source`・`datasheet` を集める。
fn source_uses<'a>(ctx: &Ctx<'a>) -> Vec<SourceUse<'a>> {
    let design = ctx.design;
    let mut out = Vec::new();
    for part in design.parts.values() {
        let id = part.id.as_str();
        for (attr, v) in &part.attr {
            let target = |state: Option<&str>| {
                Target::Param(ParamPath::Attr {
                    part: id.to_owned(),
                    attr: attr.to_string(),
                    state: state.map(str::to_owned),
                })
            };
            let entity = EntityKey::Part(id.to_owned());
            value_uses(&mut out, &entity, attr.as_str(), v, None, target);
        }
    }
    for item in design.catalog.values() {
        let id = item.id.as_str();
        let entity = EntityKey::Catalog(item.id.clone());
        let whole = Target::Catalog {
            id: item.id.clone(),
            attr: None,
        };
        for (key, field) in [("source", &item.source), ("datasheet", &item.datasheet)] {
            if let Some(source) = field.as_deref() {
                out.push(SourceUse {
                    entity: entity.clone(),
                    keys: vec![key.to_owned()],
                    target: whole.clone(),
                    source,
                    item: Some(id),
                    datasheet: key == "datasheet",
                });
            }
        }
        for (attr, v) in &item.attr {
            let target = |state: Option<&str>| Target::Catalog {
                id: item.id.clone(),
                attr: Some(AttrPath {
                    attr: attr.to_string(),
                    state: state.map(str::to_owned),
                }),
            };
            value_uses(&mut out, &entity, attr.as_str(), v, Some(id), target);
        }
    }
    out
}

/// 値（と状態ごとのセル）の `from.source`。`target` は状態（セルなら Some）→ 対象。
fn value_uses<'a>(
    out: &mut Vec<SourceUse<'a>>,
    entity: &EntityKey,
    attr: &str,
    value: &'a Authored<Value>,
    item: Option<&'a str>,
    target: impl Fn(Option<&str>) -> Target,
) {
    let mut push = |state: Option<&str>, from: Option<&'a crate::value::Provenance>| {
        if let Some(source) = from.and_then(|f| f.source.as_deref()) {
            out.push(SourceUse {
                entity: entity.clone(),
                keys: vec!["attr".to_owned(), attr.to_owned()],
                target: target(state),
                source,
                item,
                datasheet: false,
            });
        }
    };
    push(None, value.from.as_ref());
    if let Value::ByState(cells) = &value.value {
        for (state, cell) in cells {
            push(Some(state.as_str()), cell.from.as_ref());
        }
    }
}

/// 参照した出典が `sources/` にあるか。
fn check_refs(ctx: &mut Ctx, uses: &[SourceUse]) {
    for u in uses {
        if ctx.design.sources.contains_key(u.source) {
            continue;
        }
        let keys: Vec<&str> = u.keys.iter().map(String::as_str).collect();
        let shared = ctx.design.shared_catalog_of(u.source);
        // 以前の書き方（datasheet に URL・パスを直接書く）は壊さずに警告して、出典へ移すよう促す
        if u.datasheet && shared.is_none() && is_location(u.source) {
            ctx.warning(
                "source.datasheet_location",
                &u.entity,
                &keys,
                Some(u.target.clone()),
                format!(
                    "datasheet に URL・パス `{}` を直接書いています。出典（sources/）を作って ref に URL を書き、datasheet にはその出典の ID を書いてください",
                    u.source
                ),
            );
            continue;
        }
        let message = if let Some(c) = shared {
            format!(
                "出典 `{}` が共有カタログ `{}` の sources/ にありません（共有カタログの出典は `<カタログ名>/<ID>` で参照します）",
                u.source, c.name
            )
        } else if u.datasheet {
            format!(
                "データシートの出典 `{}` が sources/ にありません（datasheet には出典の ID を書き、URL・パスはその出典の ref に書きます）",
                u.source
            )
        } else {
            format!("出典 `{}` が sources/ にありません", u.source)
        };
        ctx.error(
            "ref.source",
            &u.entity,
            &keys,
            Some(u.target.clone()),
            message,
        );
    }
}

/// 書き込めないカタログ（共有カタログ）の品が、公開できない（`distribution = "restricted"`）出典を使っていないか。
fn check_restricted(ctx: &mut Ctx, uses: &[SourceUse]) {
    let design = ctx.design;
    for u in uses {
        let Some(item) = u.item.filter(|i| design.shared_catalog_of(i).is_some()) else {
            continue;
        };
        if !design
            .sources
            .get(u.source)
            .is_some_and(|s| s.is_restricted())
        {
            continue;
        }
        let keys: Vec<&str> = u.keys.iter().map(String::as_str).collect();
        ctx.error(
            "source.restricted",
            &u.entity,
            &keys,
            Some(u.target.clone()),
            format!(
                "共有カタログの品 `{item}` が公開できない出典 `{}`（distribution = \"restricted\"）を使っています",
                u.source
            ),
        );
    }
}
