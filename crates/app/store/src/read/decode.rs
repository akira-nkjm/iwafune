//! ファイル群 → Design（構文と形の誤りは `load_diagnostics` に残して、そのファイルを読み飛ばす）。
//!
//! 形の解釈は serde の derive（model の型）と、属性の値だけは [`crate::codec`] が持つ。

use super::files::FileSet;
use super::{profile, spans};
use crate::layout::{self, FileRole};
use fune_model::design::{AnalysisSystem, Design, Project};
use fune_model::id::Id;
use fune_model::{Diagnostic, Severity, Target};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

pub(super) struct Decoder<'f> {
    pub(super) design: Design,
    pub(super) files: &'f FileSet,
    /// 部品 ID → `parts/` からのディレクトリの列
    pub(super) part_dirs: BTreeMap<String, Vec<String>>,
}

/// ファイル群を Design にする（revision は呼び出し側が入れる）。
pub fn decode(files: &FileSet) -> Design {
    let mut d = Decoder {
        design: Design::default(),
        files,
        part_dirs: BTreeMap::new(),
    };
    d.index_spans();
    d.read_project();
    let catalogs: Vec<(String, String)> = d
        .design
        .project
        .catalogs
        .iter()
        .filter_map(|c| {
            c.path
                .clone()
                .map(|p| (c.name.clone(), p.trim_end_matches('/').to_string()))
        })
        .collect();
    let mut roles: Vec<(FileRole, &str)> = files
        .paths()
        .filter(|p| p.ends_with(".toml"))
        .map(|p| (layout::classify(p, &catalogs), p))
        .collect();
    // 属性定義を先に読む（値の型で文字列の読み方が決まる）。
    roles.sort_by_key(|(role, _)| match role {
        FileRole::Project => 0,
        FileRole::Attribute(_) | FileRole::Kind(_) => 1,
        _ => 2,
    });
    // 読み込みの診断には、ファイルが書いているものを対象として付ける（`parts/` のファイルならその部品）。
    let targets: BTreeMap<&str, Target> = roles
        .iter()
        .filter_map(|(role, path)| Some((*path, layout::file_target(role)?)))
        .collect();
    for (role, path) in roles {
        if let Some(entity) = layout::entity_key(&role) {
            d.design.source_map.insert_file(entity, path);
        }
        d.read_file(&role, path);
    }
    d.build_tree();
    for diag in &mut d.design.load_diagnostics {
        if diag.target.is_none() {
            diag.target = diag.file.as_deref().and_then(|f| targets.get(f)).cloned();
        }
    }
    d.design
}

impl Decoder<'_> {
    pub(super) fn diag(
        &mut self,
        severity: Severity,
        code: &str,
        file: &str,
        line: Option<u32>,
        message: String,
    ) {
        let mut diag = Diagnostic::new(severity, code, message);
        diag.file = Some(file.to_string());
        diag.line = line;
        self.design.load_diagnostics.push(diag);
    }

    /// すべての TOML の構文を確かめ、キーの位置を索引にする。
    fn index_spans(&mut self) {
        let files = self.files;
        for (path, text) in files.iter().filter(|(p, _)| p.ends_with(".toml")) {
            match toml_edit::Document::parse(text.to_string()) {
                Ok(doc) => {
                    for (key, line, column) in spans::index(&doc) {
                        self.design.source_map.insert_key(path, key, line, column);
                    }
                    for (severity, code, line, message) in profile::findings(path, &doc) {
                        self.diag(severity, code, path, (line > 0).then_some(line), message);
                    }
                }
                Err(e) => {
                    let line = e
                        .span()
                        .map(|s| spans::LineIndex::new(text).position(s.start).0);
                    self.diag(
                        Severity::Error,
                        "toml.syntax",
                        path,
                        line,
                        e.message().to_string(),
                    );
                }
            }
        }
    }

    /// serde で読む。誤りは診断にして None。
    pub(super) fn parse<T: DeserializeOwned>(&mut self, path: &str) -> Option<T> {
        let text = self.files.get(path)?;
        if toml_edit::Document::parse(text.to_string()).is_err() {
            return None; // 構文の誤りは index_spans が報告済み
        }
        match toml::from_str::<T>(text) {
            Ok(v) => Some(v),
            Err(e) => {
                let line = e
                    .span()
                    .map(|s| spans::LineIndex::new(text).position(s.start).0);
                self.diag(
                    Severity::Error,
                    "toml.schema",
                    path,
                    line,
                    e.message().to_string(),
                );
                None
            }
        }
    }

    fn read_project(&mut self) {
        if !self.files.contains(layout::PROJECT_FILE) {
            self.diag(
                Severity::Error,
                "layout.no_project",
                layout::PROJECT_FILE,
                None,
                "fune.toml がありません".into(),
            );
            return;
        }
        if let Some(p) = self.parse::<Project>(layout::PROJECT_FILE) {
            self.design.project = p;
        }
    }

    /// ファイルに `id` が書かれていれば、ファイル名の ID と同じか確かめる（ID はファイル名が正）。
    fn check_id_field(&mut self, path: &str, expected: &str) {
        let Some(table) = self
            .files
            .get(path)
            .and_then(|t| t.parse::<toml::Table>().ok())
        else {
            return;
        };
        let Some(written) = table.get("id") else {
            return;
        };
        if written.as_str() != Some(expected) {
            let line = self.design.source_map.locate_in_file(path, &["id"]).line;
            self.diag(
                Severity::Error,
                "id.mismatch",
                path,
                line,
                format!("`id = {written}` がファイル名の ID `{expected}` と違います（ID はファイル名で決まります。`id` の行は消してください）"),
            );
        }
    }

    /// serde で読んで Design に置く（誤りは診断にして読み飛ばす）。
    fn read_into<T: DeserializeOwned>(&mut self, path: &str, put: impl FnOnce(&mut Design, T)) {
        if let Some(v) = self.parse::<T>(path) {
            put(&mut self.design, v);
        }
    }

    fn read_file(&mut self, role: &FileRole, path: &str) {
        use fune_model::design as d;
        if let Some(expected) = file_id(role) {
            self.check_id_field(path, expected);
        }
        match role {
            FileRole::Project | FileRole::Reserved => {}
            FileRole::Allocation => self.read_into(path, |g, v| g.budget.allocation = v),
            FileRole::Policy => self.read_into(path, |g, v| g.budget.policy = v),
            FileRole::Attribute(id) => self.read_into(path, |g, mut v: d::AttributeDef| {
                v.id = Id::new(id);
                g.attributes.insert(Id::new(id), v);
            }),
            FileRole::Kind(id) => self.read_into(path, |g, mut v: d::KindDef| {
                v.id = Id::new(id);
                g.kinds.insert(Id::new(id), v);
            }),
            FileRole::Catalog { source, id } => self.read_catalog(path, &format!("{source}/{id}")),
            FileRole::Part { dirs, id } => self.read_part(path, dirs, id),
            FileRole::Mode(id) => self.read_into(path, |g, mut v: d::Mode| {
                v.id = Id::new(id);
                g.modes.insert(Id::new(id), v);
            }),
            FileRole::Net(id) => self.read_into(path, |g, mut v: d::Net| {
                v.id = Id::new(id);
                g.nets.insert(Id::new(id), v);
            }),
            FileRole::Harness(id) => self.read_into(path, |g, mut v: d::Harness| {
                v.id = Id::new(id);
                g.harnesses.insert(Id::new(id), v);
            }),
            FileRole::Case(id) => self.read_into(path, |g, mut v: d::Case| {
                v.id = Id::new(id);
                g.cases.insert(Id::new(id), v);
            }),
            FileRole::Source(id) => self.read_into(path, |g, mut v: d::Source| {
                v.id = Id::new(id);
                g.sources.insert(Id::new(id), v);
            }),
            FileRole::Maneuver(id) => self.read_into(path, |g, mut v: d::Maneuver| {
                v.id = Id::new(id);
                g.maneuvers.insert(Id::new(id), v);
            }),
            FileRole::Conditions(name) => self.read_conditions(path, name),
            FileRole::Ground(id) => self.read_into(path, |g, mut v: d::GroundStationSettings| {
                v.id = id.clone();
                g.conditions.ground.insert(id.clone(), v);
            }),
            FileRole::Analysis(system) => self.read_analysis(path, *system),
            FileRole::CommonAnalysis => self.design.analysis.common = self.parse(path),
            FileRole::View(id) => self.read_into(path, |g, mut v: d::ViewLayout| {
                v.id = id.clone();
                g.views.insert(id.clone(), v);
            }),
            FileRole::Other => self.diag(
                Severity::Warning,
                "layout.unknown_file",
                path,
                None,
                "このファイルはディレクトリの規約（docs/model.md §11）のどこにも当たらないので読みません".into(),
            ),
        }
    }

    fn read_conditions(&mut self, path: &str, name: &str) {
        match name {
            "orbit" => self.design.conditions.orbit = self.parse(path),
            "launch" => self.design.conditions.launch = self.parse(path),
            _ => {}
        }
    }

    fn read_analysis(&mut self, path: &str, system: AnalysisSystem) {
        match system {
            AnalysisSystem::Power => self.design.analysis.power = self.parse(path),
            AnalysisSystem::Aocs => self.design.analysis.aocs = self.parse(path),
            AnalysisSystem::Cdh => self.design.analysis.cdh = self.parse(path),
            AnalysisSystem::Comm => self.design.analysis.comm = self.parse(path),
            AnalysisSystem::Thermal => self.design.analysis.thermal = self.parse(path),
            AnalysisSystem::Structure => self.design.analysis.structure = self.parse(path),
            AnalysisSystem::Payload => self.design.analysis.payload = self.parse(path),
            AnalysisSystem::Propulsion => self.design.analysis.propulsion = self.parse(path),
        }
    }
}

/// 1 ファイル 1 エンティティで、ID がファイル名で決まる役割の ID。
fn file_id(role: &FileRole) -> Option<&str> {
    match role {
        FileRole::Attribute(id)
        | FileRole::Kind(id)
        | FileRole::Mode(id)
        | FileRole::Net(id)
        | FileRole::Harness(id)
        | FileRole::Case(id)
        | FileRole::Source(id)
        | FileRole::Maneuver(id)
        | FileRole::Ground(id)
        | FileRole::View(id) => Some(id),
        _ => None,
    }
}
