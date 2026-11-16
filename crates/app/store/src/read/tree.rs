//! 部品とカタログ品のファイルの読み込みと、ディレクトリの規則による製品ツリーの組み立て。

use super::decode::Decoder;
use crate::codec::decode_value;
use crate::layout;
use fune_model::Severity;
use fune_model::design::{
    CatalogItem, Group, Maturity, Mount, NetKind, NodeRef, Part, PortBinding, Pose,
};
use fune_model::id::Id;
use fune_model::value::{Authored, Value};
use serde::Deserialize;
use std::collections::BTreeMap;

/// 部品ファイルの形。見出しのキーはモデルのフィールド名（`PartFieldName`）、表は書き出しと同じ `PART_SECTIONS`
/// （どちらもテストで突き合わせる）。値の型はモデルの serde の型（`Mount`・`Pose`・`PortBinding`・`Maturity`）。
/// 属性の値だけは定義の型で読む（`codec`）ので、モデルの `Part` をそのまま読まない。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartDoc {
    label: Option<String>,
    description: Option<String>,
    catalog: Option<String>,
    kind: Option<String>,
    #[serde(default)]
    candidates: Vec<String>,
    count: Option<u32>,
    #[serde(default)]
    redundancy: fune_model::design::Redundancy,
    maturity: Option<Maturity>,
    mount: Option<Mount>,
    pose: Option<Pose>,
    #[serde(default)]
    mount_points: BTreeMap<String, Pose>,
    #[serde(default)]
    attr: BTreeMap<String, toml::Value>,
    #[serde(default)]
    ports: BTreeMap<String, PortBinding>,
}

/// カタログ品のファイルの形。見出しのキーはモデルのフィールド名（`CatalogFieldName`。テストで突き合わせる）。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogDoc {
    kind: String,
    label: Option<String>,
    label_en: Option<String>,
    manufacturer: Option<String>,
    part_number: Option<String>,
    maturity: Option<Maturity>,
    #[serde(default)]
    tags: Vec<String>,
    datasheet: Option<String>,
    source: Option<String>,
    #[serde(default)]
    attr: BTreeMap<String, toml::Value>,
    #[serde(default)]
    ports: BTreeMap<String, NetKind>,
}

impl Decoder<'_> {
    /// `[attr]` の値を読む（読めない値は診断にして飛ばす）。
    pub(super) fn read_attrs(
        &mut self,
        path: &str,
        raw: BTreeMap<String, toml::Value>,
    ) -> BTreeMap<Id<fune_model::design::AttributeDef>, Authored<Value>> {
        let mut out = BTreeMap::new();
        for (attr, value) in raw {
            let hint = self.design.attribute(&attr).map(|d| d.value_type);
            match decode_value(&value, hint) {
                Ok(v) => {
                    out.insert(Id::new(&attr), v);
                }
                Err(message) => {
                    let line = self
                        .design
                        .source_map
                        .locate_in_file(path, &["attr", &attr])
                        .line;
                    self.diag(
                        Severity::Error,
                        "value.syntax",
                        path,
                        line,
                        format!("attr.{attr}: {message}"),
                    );
                }
            }
        }
        out
    }

    pub(super) fn read_catalog(&mut self, path: &str, reference: &str) {
        let Some(doc) = self.parse::<CatalogDoc>(path) else {
            return;
        };
        let attr = self.read_attrs(path, doc.attr);
        let item = CatalogItem {
            id: reference.to_string(),
            kind: Id::new(doc.kind),
            label: doc.label,
            label_en: doc.label_en,
            manufacturer: doc.manufacturer,
            part_number: doc.part_number,
            maturity: doc.maturity,
            tags: doc.tags,
            datasheet: doc.datasheet,
            source: doc.source,
            attr,
            ports: doc.ports,
        };
        self.design.catalog.insert(reference.to_string(), item);
    }

    pub(super) fn read_part(&mut self, path: &str, dirs: &[String], id: &str) {
        let Some(doc) = self.parse::<PartDoc>(path) else {
            return;
        };
        if self.design.parts.contains_key(id) {
            self.diag(
                Severity::Error,
                "id.duplicate",
                path,
                None,
                format!("Part `{id}` is also defined in another file"),
            );
            return;
        }
        let attr = self.read_attrs(path, doc.attr);
        let part = Part {
            id: Id::new(id),
            parent: None,
            label: doc.label,
            description: doc.description,
            catalog: doc.catalog,
            kind: doc.kind.map(Id::new),
            candidates: doc.candidates,
            count: doc.count.unwrap_or(1),
            redundancy: doc.redundancy,
            maturity: doc.maturity,
            mount: doc.mount,
            pose: doc.pose,
            mount_points: doc.mount_points,
            attr,
            ports: doc.ports,
        };
        self.part_dirs.insert(id.to_string(), dirs.to_vec());
        self.design.parts.insert(Id::new(id), part);
    }

    /// ディレクトリの規則で製品ツリーを組み立てる（同名ファイルの無いディレクトリは Group）。
    pub(super) fn build_tree(&mut self) {
        let dirs_by_part = std::mem::take(&mut self.part_dirs);
        for (id, dirs) in &dirs_by_part {
            let mut parent: Option<NodeRef> = None;
            for (i, dir) in dirs.iter().enumerate() {
                let assembly = layout::part_path(&dirs[..i], dir);
                let node = if self.files.contains(&assembly)
                    && self.design.parts.contains_key(dir.as_str())
                {
                    NodeRef::Part(Id::new(dir))
                } else {
                    self.design
                        .groups
                        .entry(Id::new(dir))
                        .or_insert_with(|| Group {
                            id: Id::new(dir),
                            parent: parent.clone(),
                        });
                    NodeRef::Group(Id::new(dir))
                };
                parent = Some(node);
            }
            if let Some(part) = self.design.parts.get_mut(id.as_str()) {
                part.parent = parent;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::part::PART_SECTIONS;
    use fune_model::design::{CatalogFieldName, Net, NetFieldName, PartFieldName};
    use serde::de::{self, Visitor};

    /// serde の構造体が受け取るキー（`deserialize_struct` に渡る `fields`）。
    fn struct_keys<T: for<'de> Deserialize<'de>>() -> Vec<&'static str> {
        #[derive(Debug)]
        struct Stop;
        impl std::fmt::Display for Stop {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("stop")
            }
        }
        impl std::error::Error for Stop {}
        impl de::Error for Stop {
            fn custom<M: std::fmt::Display>(_: M) -> Self {
                Stop
            }
        }
        struct Grab<'a>(&'a mut Vec<&'static str>);
        impl<'de> de::Deserializer<'de> for Grab<'_> {
            type Error = Stop;
            fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Stop> {
                Err(Stop)
            }
            fn deserialize_struct<V: Visitor<'de>>(
                self,
                _: &'static str,
                fields: &'static [&'static str],
                _: V,
            ) -> Result<V::Value, Stop> {
                self.0.extend_from_slice(fields);
                Err(Stop)
            }
            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
                option unit unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
            }
        }
        let mut keys = Vec::new();
        let _ = T::deserialize(Grab(&mut keys));
        keys
    }

    /// 読み込みが受け取るキー = モデルのフィールド名の一覧（正規の順）+ 表の見出し（書き出しと同じ名前と順）。
    #[test]
    fn file_keys_are_the_model_field_names() {
        let part: Vec<&str> = PartFieldName::NAMES
            .iter()
            .chain(PART_SECTIONS)
            .copied()
            .collect();
        assert_eq!(struct_keys::<PartDoc>(), part);
        let catalog: Vec<&str> = CatalogFieldName::NAMES
            .iter()
            .chain(&["attr", "ports"])
            .copied()
            .collect();
        assert_eq!(struct_keys::<CatalogDoc>(), catalog);
        // ネットのファイルはモデルの `Net` をそのまま読む（`id` はファイル名）
        let net: Vec<&str> = struct_keys::<Net>()
            .into_iter()
            .filter(|k| *k != "id")
            .collect();
        assert_eq!(net, NetFieldName::NAMES);
    }
}
