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

/// 部品ファイルの形。
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

/// カタログ品のファイルの形。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogDoc {
    kind: String,
    label: Option<String>,
    label_en: Option<String>,
    manufacturer: Option<String>,
    part_number: Option<String>,
    maturity: Option<Maturity>,
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
                format!("部品 `{id}` は別のファイルにもあります"),
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
                    let label = self.design.project.groups.get(dir).cloned();
                    self.design
                        .groups
                        .entry(Id::new(dir))
                        .or_insert_with(|| Group {
                            id: Id::new(dir),
                            parent: parent.clone(),
                            label,
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
