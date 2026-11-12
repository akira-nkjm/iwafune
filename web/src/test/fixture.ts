import type {
  AttributeDefView,
  DesignView,
  NetKind,
  NetView,
  PartRow,
  PortRef,
  PortView,
  QuantityView,
  Role,
  ValueData,
  ValueView,
} from "@/api/types";

/** テスト用の小さな設計（GET /design の形）。表示文字列は service が作る想定の値を手で入れてある */
export function qv(value: number, unit: string, dimension: string, si = value): QuantityView {
  return { value, unit, si, dimension, display: unit ? `${value} ${unit}` : String(value) };
}

function val(attr: string, value: ValueData, display: string): ValueView {
  return {
    attr,
    value,
    display,
    origin: "catalog",
    evidence: "datasheet",
    source: null,
    note: null,
    file: null,
    line: null,
  };
}

const scalar = (attr: string, q: QuantityView) =>
  val(attr, { type: "scalar", nominal: q, min: null, max: null }, q.display);

function part(
  id: string,
  group: PartRow["system"] & string,
  kind: Role,
  states: string[],
  values: ValueView[],
  ports: PortView[] = [],
): PartRow {
  return {
    id,
    paths: {},
    field_paths: {},
    parent: group,
    system: group,
    kind,
    role: kind,
    catalog: null,
    candidates: [],
    count: 1,
    maturity: "off_the_shelf",
    values: Object.fromEntries(values.map((v) => [v.attr, v])),
    ports,
    mount: null,
    states,
    file: `parts/${group}/${id}.toml`,
    dir: [group],
  };
}

function net(id: string, kind: NetKind, members: PortRef[], extra: Partial<NetView> = {}): NetView {
  return {
    id,
    kind,
    is_return: false,
    label: id,
    source: null,
    rail: null,
    limit: null,
    protocol: null,
    bitrate: null,
    master: null,
    members,
    field_paths: { rail: `nets.${id}.rail` },
    file: `nets/${id}.toml`,
    ...extra,
  };
}

export const fixture: DesignView = {
  project: {
    name: "demo-6u",
    root: "/tmp/demo-6u",
    revision: "rev1",
    branch: "main",
    schema: 2,
    counts: { parts: 3, groups: 3, modes: 2, nets: 2, harnesses: 0, cases: 0 },
    diagnostics: { errors: 0, warnings: 1, infos: 0 },
    catalogs: [{ name: "local", editable: true }],
  },
  tree: [
    {
      id: "aocs",
      node: "group",
      label_en: "AOCS",
      system: "aocs",
      label: "AOCS",
      parent: null,
      children: ["RW_X"],
    },
    {
      id: "power",
      node: "group",
      label_en: "Power",
      system: "power",
      label: "電源系",
      parent: null,
      children: ["PCDU"],
    },
    {
      id: "comm",
      node: "group",
      label_en: "Communications",
      system: "comm",
      label: "通信系",
      parent: null,
      children: ["TRX"],
    },
    {
      id: "RW_X",
      node: "part",
      label_en: null,
      system: "aocs",
      label: "RW_X",
      parent: "aocs",
      children: [],
    },
    {
      id: "PCDU",
      node: "part",
      label_en: null,
      system: "power",
      label: "PCDU",
      parent: "power",
      children: [],
    },
    {
      id: "TRX",
      node: "part",
      label_en: null,
      system: "comm",
      label: "TRX",
      parent: "comm",
      children: [],
    },
  ],
  parts: [
    part(
      "RW_X",
      "aocs",
      "reaction_wheel",
      ["off", "idle", "on", "peak"],
      [
        scalar("mass", qv(0.62, "kg", "Mass")),
        val(
          "power",
          { type: "by_state", states: { off: qv(0, "W", "Power"), on: qv(1, "W", "Power") } },
          "off 0 W / on 1 W",
        ),
        scalar("max_momentum", qv(18, "mN*m*s", "AngularMomentum")),
      ],
      [
        { id: "PWR", net: "P12V" },
        { id: "DATA", net: "CAN" },
      ],
    ),
    part(
      "PCDU",
      "power",
      "pcdu",
      ["off", "on"],
      [scalar("mass", qv(0.3, "kg", "Mass"))],
      [
        { id: "OUT3", net: "P12V" },
        { id: "CAN1", net: "CAN" },
      ],
    ),
    part(
      "TRX",
      "comm",
      "transceiver",
      ["off", "rx", "tx"],
      [scalar("tx_power", qv(1, "W", "Power"))],
      [
        { id: "PWR", net: "P12V" },
        { id: "CAN", net: "CAN" },
        { id: "RF", net: null },
      ],
    ),
  ],
  modes: {
    modes: [
      { id: "NOMINAL", label: "定常", description: null, file: "modes/NOMINAL.toml" },
      { id: "SAFE", label: "セーフ", description: null, file: "modes/SAFE.toml" },
    ],
    rows: [
      {
        part: "RW_X",
        kind: "reaction_wheel",
        states: ["off", "idle", "on", "peak"],
        cells: { NOMINAL: { state: "on", duty: {} } },
        shares: {},
      },
      {
        part: "PCDU",
        kind: "pcdu",
        states: ["off", "on"],
        cells: { NOMINAL: { state: "on", duty: {} }, SAFE: { state: "on", duty: {} } },
        shares: {},
      },
      {
        part: "TRX",
        kind: "transceiver",
        states: ["off", "rx", "tx"],
        cells: { NOMINAL: { state: "rx", duty: { tx: 0.1 } } },
        shares: { NOMINAL: { tx: qv(10, "%", "Dimensionless", 0.1) } },
      },
    ],
  },
  nets: [
    net(
      "P12V",
      "power",
      [
        { part: "PCDU", port: "OUT3" },
        { part: "RW_X", port: "PWR" },
        { part: "TRX", port: "PWR" },
      ],
      {
        label: "12 V",
        source: { part: "PCDU", port: "OUT3" },
        rail: qv(12, "V", "Voltage"),
      },
    ),
    net(
      "CAN",
      "data",
      [
        { part: "PCDU", port: "CAN1" },
        { part: "RW_X", port: "DATA" },
        { part: "TRX", port: "CAN" },
      ],
      {
        protocol: "CAN",
        master: { part: "PCDU", port: "CAN1" },
      },
    ),
  ],
  harnesses: [],
  defs: {
    attributes: [
      ["power", "power"],
      ["max_momentum", "aocs"],
      ["tx_power", "comm"],
    ].map(
      ([id, section]): AttributeDefView => ({
        id: id ?? "",
        section: section ?? "",
        label: id ?? "",
        label_en: null,
        value_type: "scalar",
        dimension: null,
        units: [],
        display_unit: null,
        by_state: false,
        builtin: true,
        description: null,
      }),
    ),
    kinds: [],
    labels: {
      role_system: { pcdu: "power", reaction_wheel: "aocs", transceiver: "comm" },
      section_order: [],
      state: {},
      section: {
        aocs: { label: "AOCS", label_en: "AOCS" },
        power: { label: "電源", label_en: "Power" },
        comm: { label: "通信", label_en: "Communications" },
      },
      enum: {},
      enum_order: {},
    },
    systems: ["power", "aocs", "cdh", "comm", "thermal", "structure", "payload"],
    budget_kinds: ["mass", "power", "data"],
  },
  settings: [],
  cases: [],
  views: [],
  sources: [],
  catalog: [],
  diagnostics: [],
};
