import type {
  AttributeDefView,
  BodyPoseView,
  DesignView,
  MountView,
  NetKind,
  NetView,
  PartRow,
  PortRef,
  PortView,
  PoseQuantities,
  QuantityView,
  Role,
  ValueData,
  ValueView,
} from "@/api/types";

/** テスト用の小さな設計（GET /design の形）。表示文字列は service が作る想定の値を手で入れてある */
export function qv(value: number, unit: string, dimension: string, si = value): QuantityView {
  return { value, unit, si, dimension, display: unit ? `${value} ${unit}` : String(value) };
}

type Xyz = [number, number, number];
type Wxyz = [number, number, number, number];

/** 位置・姿勢の表示（原点・恒等。位置は m、角度は度） */
export function zeroPose(): PoseQuantities {
  const len = qv(0, "m", "Length");
  const deg = { ...qv(0, "deg", "Angle"), display: "0°" };
  return { position: [len, len, len], rotation: [deg, deg, deg] };
}

/** 取付（取付点なし・書かれた位置と姿勢なし。表示は原点・恒等） */
export function mountAt(
  on: string,
  position: Xyz = [0, 0, 0],
  orientation: Wxyz = [1, 0, 0, 0],
): MountView {
  return {
    on,
    point: null,
    position,
    authored_position: null,
    orientation,
    authored_orientation: null,
    overridden: false,
    point_pose: zeroPose(),
    relative: zeroPose(),
  };
}

/** 機体座標の位置・姿勢（表示は原点・恒等） */
export function bodyAt(position: Xyz, orientation: Wxyz, root = false): BodyPoseView {
  return { position, orientation, root, quantities: zeroPose() };
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
    maneuver_thruster: false,
    structural: group === "structure",
    diagnostics: { errors: 0, warnings: 0, infos: 0 },
    unset_count: 0,
    catalog: null,
    candidates: [],
    count: 1,
    redundancy: "none",
    maturity: "off_the_shelf",
    values: Object.fromEntries(values.map((v) => [v.attr, v])),
    ports,
    mount: null,
    mount_points: [],
    body: null,
    states,
    file: `parts/${group}/${id}.toml`,
    dir: [group],
  };
}

function net(id: string, kind: NetKind, members: PortRef[], extra: Partial<NetView> = {}): NetView {
  // 系統と中心は API が決める。ここでは、供給元・マスターが書かれていればそのポートを中心にする
  const declared = kind === "power" ? extra.source : extra.master;
  return {
    id,
    kind,
    is_return: false,
    power_class: kind === "power" ? (extra.rail ? "regulated" : "unregulated") : null,
    hub: declared ? { port: declared, collects: false } : null,
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
    catalogs: [{ name: "local", editable: true, shared: null }],
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
      {
        id: "NOMINAL",
        label: "定常",
        description: null,
        max_duration: null,
        when: null,
        start: null,
        file: "modes/NOMINAL.toml",
      },
      {
        id: "SAFE",
        label: "セーフ",
        description: null,
        max_duration: null,
        when: null,
        start: null,
        file: "modes/SAFE.toml",
      },
    ],
    rows: [
      {
        part: "RW_X",
        kind: "reaction_wheel",
        states: ["off", "idle", "on", "peak"],
        off_state: "off",
        main_state: "on",
        cells: { NOMINAL: { state: "on", duty: {} } },
        eclipse_cells: {},
        shares: {},
      },
      {
        part: "PCDU",
        kind: "pcdu",
        states: ["off", "on"],
        off_state: "off",
        main_state: "on",
        cells: { NOMINAL: { state: "on", duty: {} }, SAFE: { state: "on", duty: {} } },
        eclipse_cells: {},
        shares: {},
      },
      {
        part: "TRX",
        kind: "transceiver",
        states: ["off", "rx", "tx"],
        off_state: "off",
        main_state: "rx",
        cells: { NOMINAL: { state: "rx", duty: { tx: 0.1 } } },
        eclipse_cells: {},
        shares: { NOMINAL: { tx: qv(10, "%", "Dimensionless", 0.1) } },
      },
    ],
    sequences: [],
    active_sequence: null,
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
    ]
      .map(
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
          unit_vector: false,
          fov_axis: null,
          builtin: true,
          description: null,
          input: { kind: "quantity", dimension: null, by_state: false },
        }),
      )
      .concat(
        [
          ["axis", true, null],
          ["boresight", true, null],
          ["direction", true, null],
          ["normal", true, null],
          ["spin_axis", true, null],
          ["fov_half_angle", false, "boresight"],
        ].map(
          ([id, unitVector, fovAxis]): AttributeDefView => ({
            id: String(id),
            section: "aocs",
            label: String(id),
            label_en: null,
            value_type: unitVector ? "vec3" : "scalar",
            dimension: null,
            units: [],
            display_unit: null,
            by_state: false,
            unit_vector: unitVector === true,
            fov_axis: typeof fovAxis === "string" ? fovAxis : null,
            builtin: true,
            description: null,
            input: unitVector
              ? { kind: "read_only", reason: "vector" }
              : { kind: "quantity", dimension: null, by_state: false },
          }),
        ),
      ),
    kinds: [],
    labels: {
      role_other: null,
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
    systems: ["aocs", "cdh", "comm", "payload", "power", "propulsion", "structure", "thermal"],
    budget_kinds: ["mass", "power", "data"],
    mode_systems: ["aocs", "cdh", "power", "thermal"],
    mode_levels: [
      { system: null, levels: "power_levels" },
      { system: "aocs", levels: "states" },
      { system: "cdh", levels: "states" },
      { system: "power", levels: "power_levels" },
      { system: "thermal", levels: "states" },
    ],
  },
  settings: [],
  cases: [],
  maneuvers: [],
  phases: [],
  configurations: [],
  stations: [],
  requirements: [],
  views: [],
  sources: [],
  catalog: [],
  diagnostics: [],
};
