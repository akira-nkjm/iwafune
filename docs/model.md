# 設計モデルの構造とファイル形式

設計データのモデルをどんな構造体で持つか、ファイルにどう置くかを書く。

## 1. 結論

- **モデルは 4 つの層に分ける。** ファイル層 → Design（書かれたまま）→ Resolved（解決済み・SI・由来付き）→ Derived（計算結果）。
  編集は Design に対して行い、解析は Resolved を読む。Derived は原則ファイルに保存しない（§9）。
- **解析はモデルを知らない。** 解析ライブラリ側が「解析ごとの入力型」を定義し、fune-projection が Resolved からその型へ射影して渡す
  （[architecture.md](architecture.md) §6.3）。足りない入力は「この解析には RW_X.max_momentum が要る」と名指しで返す。
- **製品ツリーはディレクトリで表す。** 最上位は系の ID（`aocs` など）のディレクトリで、その下に組立品を入れ子にできる。`parts/` 直下の部品も許す。
  自分の質量や取付先を持つ物理的な組立品は「部品ファイル＋同じ名前のディレクトリ」で表す（§6）。
- **置き場所の原則**（§7）:
  1 つの部品で完結する情報は部品ファイルに書く。複数の部品にまたがり、それ自体が名前や属性を持つもの（モード・ネット・ハーネス）は別ディレクトリにする。
  所属は子が親を指す（部品のポートがネットを指す）。
- **システム担当が決めるもの（配分・マージン方針）は `budget/` にまとめる**（§8）。サブシステムのディレクトリには置かない。
- **ファイル形式は TOML。書き出しは 1.0 の範囲に留める**（§10）。表データは CSV、機械向けの交換は JSON。
- **形式はコーデック層に閉じ込める。** model は TOML を知らず、store の `Codec` だけが知る。将来、形式を変えても model と解析に波及しない。

### 1.1 前提条件と解析の区分

前提条件は `conditions/orbit.toml`・`conditions/launch.toml`・`conditions/ground/<ID>.toml` に置く。
設定の区分は `SettingScope::Conditions`（綴り `conditions`、表示名は「前提条件」／「Conditions」）。
キーは `conditions.orbit.altitude`・`conditions.launch.<欄>`・`conditions.ground.<ID>.<欄>`。
`mission/` は #9 のミッション・サクセスクライテリア・フェーズ用の予約ディレクトリで、現在は読み飛ばす。
旧配置の軌道なども読まず、軌道の不足は既存の不足入力として返す。移行や旧キーの互換性は持たない。

解析の区分は `sat_analysis::spec::AnalysisArea`。`System(System)` と `Condition(ConditionTopic)` があり、
最初の話題は `Orbit`（綴り `orbit`、表示名「軌道」／「Orbit」）。既存の 16 解析はすべて系の区分のままで、
ID は `<区分のキー>.<解析名>`。話題と系のキーは重ならない。部品・バジェット・色・`parts/<系>/` は系のまま扱う。
API の `AnalysisInfo.area` は `{ "kind": "system", "value": "power" }` または
`{ "kind": "condition", "value": "orbit" }`。KPI は解析の区分に従って系／話題ごとに並べる。

話題の解析の設定はその条件のファイル内の表に置く（例: `conditions/orbit.toml` の `[eclipse_season]`、
設定キーは `conditions.orbit.eclipse_season.<欄>`）。`analysis/orbit.toml` は作らない。
話題の設定位置は `ConditionTopic::setting_path()` で引く。型付きの設定は未知の表・キーを受け入れない。

`conditions/orbit.toml` の `duration` はミッション期間（時間の量、キー `conditions.orbit.duration`）。
`[eclipse_season]` の任意の `step` も時間の量（キー `conditions.orbit.eclipse_season.step`）。
読み書きは既存の条件ファイルの経路、次元の検査と projection は `setting_dims` の共通の表を使う。
未入力の期間・刻みも設定の表に表示する。`duration` が無いと季節解析は不足入力となる。

```toml
# conditions/orbit.toml（他の軌道要素と同じファイル）
duration = { value = 1, unit = "yr" }

[eclipse_season]
step = { value = 1, unit = "day" }
```

`orbit.eclipse_season` は `AnalysisArea::Condition(ConditionTopic::Orbit)`。
入力は `OrbitSpec`・`duration`・任意の `step`。刻みの既定の1日は sat-analysis が持ち、projection は補わない。
期間の始点・終点を含む標本ごとに、既存の `beta_angle_rad` で J2 の RAAN 歳差と太陽方向を考慮した β を求める。
各標本の1周では太陽位置を固定し、既存の軌道伝搬と円錐影を5秒間隔で標本化する。
本影（光の割合 0）と、本影を含まない半影（0 より大きく1未満）の時間は、既存の時間割合の台形則で集計する。
境界の時間精度は各境界で標本間隔程度。短い蝕・半影は標本間に隠れることがある。
地球以外の中心天体は未対応のエラー。月による遮蔽（月の影）は後回しとし、含めない。

出力 `points` は日付（UTC）・β [rad]・本影 [s]・半影 [s] の列、`series` は同じ値のグラフ用の系列。
`longest` と `max_eclipse_duration` は本影と半影の合計が最大の標本と時間、
`beta_min` / `beta_max` は符号付き β の極値とその標本。同じ極値は最初の日付を選ぶ。
極値は標本の中で選び、日付間の真の極値を探索しない。
`eclipse_free_samples` は蝕の無い標本の数（端数の終点も含む）、
`eclipse_free_duration` は蝕なしフラグを季節の格子で台形積算した期間の推定値。
季節の間隔 `step`、周回内の間隔 `shadow_step`、近似 `fixed_sun_per_orbit` も出す。
判定は情報のみで checks は付けない。KPI は最長蝕と β の範囲の下端・上端を軌道の話題に並べる。

`fune case from-eclipse <新しいID> --at longest|beta_min [--mode <モード>]` は
service の `case_from_eclipse_op` で最新の条件を再解析し、既存の `SetCase` Op を作る。
HTTP は `POST /ops/case_from_eclipse` に `{ "id": "season_longest", "at": "longest", "mode": null }` を送り、
返る Op を通常の `POST /ops` で保存する。GUI の軌道ページもこの入口を使い、プレビュー・CLI 表示を共用する。
ケースには β（rad）と任意のモードを保存する。ケースは元期の欄を持たないので、標本の日付は description に記録する。
これは β の代表条件を再利用するケースであり、description の日付で後続の解析の元期を変えるものではない。
既存 ID の上書き・未知のモードは拒否する。`to_cli` は値と日付を保持した `fune case set` を返す。


## 2. 層の構成

| 層 | 中身 | 誰が作るか | 誰が使うか | 保存 |
|---|---|---|---|---|
| ファイル層 | ファイルごとの `toml_edit::DocumentMut` と位置情報 | store（Codec） | store の書き込み | git の作業ツリー |
| Design | 書かれたままの値（`0.62 kg`、`1.0 mN*m` は単位も書かれたまま）、出典、ファイル位置。型付き ID で正規化 | store の読み込み + 構造検査 | ops（編集の対象）、GUI の「元の値」表示 | ファイル層そのもの |
| Resolved | カタログ・kind・上書きを重ねた各部品の値（SI）、値の出所（catalog / override / derived）、根拠の最弱値、取付をたどった機体座標、選択中の候補 | `resolve(&Design)`（純粋関数） | 解析への射影、GUI の表、check() | しない（毎回作り直す。数千件で数十 ms） |
| Derived | 質量特性（総質量・重心・慣性）、バジェット、解析・SIM の結果 | budget、解析ライブラリ、SIM | GUI、CLI、PR コメント | 原則しない。出力先は §9 |

```mermaid
flowchart LR
  F["ファイル層<br/>TOML / CSV"] -->|"Codec.read"| D["Design<br/>書かれたまま"]
  D -->|"resolve()"| R["Resolved<br/>SI・由来付き"]
  R -->|"project()"| I["解析ごとの入力型"]
  I --> A["解析ライブラリ / SIM"]
  A --> X["Derived<br/>バジェット・解析結果"]
  O["Op（編集）"] -->|"plan / apply"| F
```

## 3. エンティティ

| エンティティ | 役割 | ファイル | 主な中身 |
|---|---|---|---|
| Project | プロジェクト全体の設定 | `fune.toml` | schema 版、書き出す TOML の版、共有カタログの参照 |
| Allocation | 配分 | `budget/allocation.toml` | 質量・電力などの上限、システムマージン、系（または部品）ごとの配分 |
| BudgetPolicy | マージン方針 | `budget/policy.toml` | 成熟度ごとの成長余裕、不確かさの既定幅、ハーネスの見込み |
| AttributeDef | 量の種類（quantity kind） | `defs/attribute/<id>.toml` | 値の型、次元、許可単位、表示単位、値域、状態で引くか、どのバジェットに入るか |
| KindDef | 部品の種別 | `defs/kind/<id>.toml` | 継承元、必須・任意の属性、機器の状態の語彙（off / standby / on …）、ポートの雛形、role |
| CatalogItem | 購入品・設計品の定義（ElementDefinition） | 共有カタログ or `catalog/<id>.toml` | kind、型番、成熟度、属性値、ポートの値、形状、データシート |
| Group | 系・入れ子のフォルダ | `parts/` 以下の、同名ファイルを持たないディレクトリ（最上位は系 ID） | なし（系の表示名は組み込みの語彙、配分は `budget/allocation.toml`） |
| Part | 製品ツリー上の使用（ElementUsage） | `parts/<…>/<ID>.toml` | 定義（catalog / kind）、トレード候補、員数・冗長、取付先と相対姿勢、上書き値、ポートの所属ネット |
| Mode | 衛星の運用モード | `modes/<ID>.toml` | 継続時間、部品 → {状態, 時間割合, 稼働台数, 予備状態}、蝕の上書き |
| Net | 電気的な接続（論理） | `nets/<ID>.toml` | 種類（power / data / rf）、レール電圧、供給元ポート、電流制限、プロトコル |
| Harness | ケーブル区間（物理） | `harness/<ID>.toml` | 両端のポート、通すネット、線材、芯数、長さ |
| Case | 解析ケース | `cases/<ID>.toml` | モードまたはモード時系列、不確かさのどちら側を読むか、軌道、姿勢則、環境条件、期間 |
| Source | 出典の台帳 | `sources/<ID>.toml` | 種類（datasheet / measured / analysis / judgement / placeholder）、参照先、日付、記入者 |
| Requirement | 要求（軽量） | `requirements/<ID>.toml` | 文、対象の量、制約、「全モード・全ケースで」などの量化。正式な判定は外部の検証ツールに書き出す |
| Table | 表データ | `tables/<ID>.csv` + `tables/<ID>.toml` | 曲線や特性表。列ごとの単位は TOML 側で宣言 |
| View | 見た目だけの情報（Design の外） | `view/…` | 系統図の座標、3D のカメラ |

### 3.1 種別の役割と状態

- 種別の `role` は `fune_model::builtin::Role`（`enum_text!`）の値。綴りは `builtin/role.rs` の 1 か所で定義し、未知の役割は serde の読み込みで拒む。省略時は継承元の役割、継承元にも無ければ役割なし。独自の種別 ID は自由だが、役割の語彙は追加しない。
- 役割の表示名は対応する組み込み種別の日英名。`labels.role_system` は Role をキーに全役割の系を持つ（系に属さない対応は `None` と明示）。API の役割は TS の `Role` 型で返し、Web のアイコン表は全役割を網羅する。
- 状態の語彙は種別ごとに自由に定義する。慣例の停止状態だけは `fune_model::builtin::STATE_OFF` を使い、既定状態を選ぶ処理でもこの定数を参照する（列挙型にはしない）。

## 4. Rust の型（Design 層）

```rust
/// 型付き ID。ファイル名（またはディレクトリ名）の stem と一致し、全体で一意（大文字小文字を無視して比較）。
pub struct Id<T> { name: SmolStr, _t: PhantomData<fn() -> T> }

/// 書かれたままの設計。不変のスナップショットで、編集のたびに新しいものを作る。
pub struct Design {
    pub project: Project,
    pub budget: Budget,                                    // budget/allocation.toml と policy.toml
    pub attributes: BTreeMap<Id<AttributeDef>, Arc<AttributeDef>>,
    pub kinds: BTreeMap<Id<KindDef>, Arc<KindDef>>,
    pub catalog: BTreeMap<CatalogRef, Arc<CatalogItem>>,   // "lab/rw_18mNms" のように出所付き
    pub groups: BTreeMap<Id<Group>, Group>,                // 同名ファイルを持たないディレクトリ
    pub parts: BTreeMap<Id<Part>, Arc<Part>>,
    pub modes: BTreeMap<Id<Mode>, Arc<Mode>>,
    pub nets: BTreeMap<Id<Net>, Arc<Net>>,
    pub harnesses: BTreeMap<Id<Harness>, Arc<Harness>>,
    pub cases: BTreeMap<Id<Case>, Arc<Case>>,
    pub sources: BTreeMap<Id<Source>, Arc<Source>>,
    pub requirements: BTreeMap<Id<Requirement>, Arc<Requirement>>,
    pub tables: BTreeMap<Id<Table>, Arc<TableDecl>>,
    pub revision: Revision,                                // 追跡ファイルの (path, bytes) の BLAKE3
}

/// 製品ツリーのノード。Group と Part の ID は同じ名前空間で一意。
pub enum NodeRef { Group(Id<Group>), Part(Id<Part>) }

pub struct Group {
    pub parent: Option<NodeRef>,           // ディレクトリから決まる
}

pub struct Part {
    pub parent: Option<NodeRef>,           // ディレクトリから決まる（ファイルには書かない）
    pub def: PartDef,                      // Catalog(CatalogRef) | Kind(Id<KindDef>)
    pub candidates: Vec<CatalogRef>,       // トレード候補。def の catalog が選択中の 1 つ
    pub count: u32,                        // 員数。個別の姿勢や配線が要るなら別の Part にする
    pub redundancy: Redundancy,            // none（既定）/ cold / warm
    pub mount: Option<Mount>,              // 取付先の Part と取付点
    pub pose: Option<Pose>,                // 取付点の座標系に対する相対位置・姿勢
    pub mount_points: BTreeMap<Id<MountPoint>, Pose>,       // 自分に他の部品を付ける点
    pub attr: BTreeMap<Id<AttributeDef>, Authored<Value>>,  // カタログ値への上書き
    pub ports: BTreeMap<Id<Port>, PortBinding>,             // ポート → 所属ネット
    pub maturity: Option<Maturity>,        // off_the_shelf / modified / new
    pub loc: Loc,
}

/// 書かれた値と、その出典と位置。
pub struct Authored<T> { pub value: T, pub from: Option<From>, pub loc: Loc }

/// 値の型は AttributeDef が宣言し、読み込み時に一致を検査する。
pub enum Value {
    Scalar(Uncertain<Quantity>),
    ByState(BTreeMap<Id<State>, Uncertain<Quantity>>),   // 機器の状態ごと（電力・発熱など）
    Vec3 { x: f64, y: f64, z: f64, unit: UnitExpr, frame: FrameRef },
    Quat { w: f64, x: f64, y: f64, z: f64 },
    Tensor3 { m: [[f64; 3]; 3], unit: UnitExpr, frame: FrameRef },
    Enum(Symbol),
    Bool(bool),
    Text(String),
    Range { min: Quantity, max: Quantity },
    Table(Id<Table>),
}

/// 書かれたままの数値と単位。mN*m は mN*m のまま持ち、SI への換算は Resolved で行う。
pub struct Quantity { pub number: f64, pub unit: UnitExpr }

/// nominal と任意の不確かさ。どちら側を読むかは解析ケースが決める。
pub struct Uncertain<T> { pub nominal: T, pub min: Option<T>, pub max: Option<T> }

pub struct Mode {
    pub label: String,
    pub description: Option<String>,
    pub states: BTreeMap<Id<Part>, StateAssign>,
    pub max_duration: Option<Quantity>,    // 時間の次元。power.energy_balance の判定に使う（§5.1）
    pub eclipse: BTreeMap<Id<Part>, StateAssign>, // 書いた部品だけ蝕時に置換
}
/// 基本の状態と、ほかの状態で過ごす時間の割合。割合の残りは基本の状態。
pub struct StateAssign {
    pub state: Id<State>,
    pub duty: BTreeMap<Id<State>, f64>,
    pub active: Option<u32>,               // 省略は全台
    pub spare_state: Option<String>,      // warm の予備状態の明示
}
pub enum Redundancy { None, Cold, Warm }

pub struct PortBinding { pub net: Id<Net> }

pub struct Net {
    pub kind: NetKind,                     // Power / Data / Rf
    pub label: Option<String>,
    pub source: Option<PortRef>,           // 電源の供給元（"PCDU.OUT3"）
    pub rail: Option<Authored<Quantity>>,
    pub limit: Option<Authored<Quantity>>,
    pub protocol: Option<Symbol>,
    pub bitrate: Option<Authored<Quantity>>,
    pub master: Option<PortRef>,
}

pub struct Harness {
    pub label: Option<String>,
    pub from: PortRef,
    pub to: PortRef,
    pub nets: Vec<Id<Net>>,
    pub wire: CatalogRef,                  // 線材（単位長さ質量・抵抗）
    pub cores: u32,
    pub length: Authored<Quantity>,
}

pub struct Budget {
    pub allocation: BTreeMap<BudgetKind, Allocation>,      // Mass / Power / Data …
    pub policy: BudgetPolicy,
}
pub struct Allocation {
    pub limit: Authored<Quantity>,
    pub system_margin: f64,
    pub alloc: BTreeMap<NodeRef, Authored<Quantity>>,
}
pub struct BudgetPolicy {
    pub growth: BTreeMap<Maturity, f64>,
    pub uncertainty: BTreeMap<Maturity, f64>,
    pub harness_allowance: f64,
}
```

## 5. Rust の型（Resolved 層と解析への射影）

```rust
pub struct Resolved {
    pub parts: BTreeMap<Id<Part>, ResolvedPart>,
    pub tree: Tree<NodeRef>,               // 製品ツリー（機能の階層）
    pub mounts: Tree<Id<Part>>,            // 取付の階層（物理の階層）
    pub diagnostics: Vec<Diagnostic>,      // severity・code・対象（Target）・ファイル位置
}

pub struct ResolvedPart {
    pub catalog: Option<CatalogRef>,       // 候補があるときは選択中のもの
    pub kind: Id<KindDef>,
    pub count: u32,
    pub attr: BTreeMap<Id<AttributeDef>, ResolvedValue>,
    pub pose_body: Option<PoseSi>,         // 取付の連鎖をたどった機体座標
}

pub struct ResolvedValue {
    pub value: Cell<SiValue>,              // 欠けていても Unset として持ち、エラーにしない
    pub origin: Origin,                    // Catalog / Override / KindDefault / Derived
    pub evidence: Evidence,                // 根拠の最弱値（placeholder が混ざれば placeholder）
    pub loc: Option<Loc>,
}
pub enum Cell<T> { Set(T), Unset }

/// 解析ライブラリの入力型への射影。解析ライブラリは設計モデルに依存しない。
pub trait Project<I> {
    fn project(r: &Resolved, case: &CaseCtx) -> Result<I, MissingInputs>;
}
// 例: impl Project<rw_unloading::Input> for RwUnloadingProjection { … }
// MissingInputs は対象（fune_model::Target）で返す。表示は "RW_X.max_momentum"・"<部品>.power" のようなパス
```

- 編集の流れ: Op は Design のパス（`RW_X.attr.mass`）を対象にする。store が最小差分のファイル編集に変え、
  書いたファイルだけ読み直して新しい Design を作り、Resolved を作り直す。
- 計算のキャッシュ: Derived は入力の fingerprint をキーにメモ化する。数千件規模なら単純な全再計算で足りる。
  遅くなったら incremental な計算フレームワーク（salsa など）を検討する。

### 5.1 解析ケースの条件選択

projection の `case_policy::effective_case` が明示ケースを優先する。省略時に `analysis.common.default_case` を
解析全体へ適用する対象は、同モジュールの宣言にある `power.orbit_generation`・`aocs.rw_unloading`・
`aocs.sensor_fov`・`thermal.transient` のみ。それ以外はケース未選択のまま運用全体や全条件を解析する。
既定の設定が存在しないケースを指すと検査エラーにする。
実行結果の `case` は使ったケース ID、`case_source` は `explicit` / `default`（ケース未選択なら原則両方 null）。
保存前の設定変更も同じ規則で解析に適用する。入力を丸ごと与える `given` には既定ケースを適用しない。

`power.energy_balance` はケース未指定なら全モードを 1 ケースずつ並べ、各ケースの姿勢則はそのモードの
`analysis.aocs.law` を使う。上段の `timeline.attitude` だけは、同じ解決関数で引いた既定ケースのモードの姿勢則を使う。
この場合は `case = null`、`case_source = default_attitude` とする。既定ケースの β・周回数・side などは適用しない。
既定ケースがなければ、姿勢則が唯一の場合だけ上段にも使い、複数なら不足入力にする。
明示ケースがあれば、従来どおりそのケースのモードと条件を使う。

電力収支の各ケースの `energy_margin_per_orbit` は、回り切った周回ごとのエネルギー余裕の最小値
（電池の効率込み、容量の上下限で切る前、周期へ換算済み）。回り切った周回が無ければ `null` とし、
`energy_margin_over_span` に解析期間全体の収支を周期へ換算せず返す。この場合の周回の収支判定は Warning とし、
判定文に期間の収支を添える。回り切った周回があれば `energy_margin_over_span` は `null`。
期間不足の目印 `span_fallback = orbit_mean_over_span` は引き続き返す。
KPI の電力収支は別の量である平均電力（`orbit_average_balance`、電池の効率を含まない）を使う。

電力収支の運用は 3 通りある。ケースの欄（`max_dod`・`orbit_average_balance`・`orbits`・`series` など）は
どれも**解析期間ずっと続けた**ときの値のまま。判定（`checks` の `dod`・`energy_balance`・`depleted`）をどの運用で
評価したかはケースの `evaluation` に書く。

- **連続**（`continuous`）: 最長継続時間の無いモード・一定の負荷。最長継続時間があっても基本のモードが無ければ
  `no_base_mode`、そのモード自身が基本のモードなら `base_mode` とし、どちらも連続で判定する。
- **最長継続時間**（`max_duration`）: モードの `max_duration` があれば、基本のモード（`analysis.power.base_mode`、
  無ければ `[[sequence]]` の最初の区間のモード）を回し続け、その上でこのモードを最長継続時間だけ 1 回続ける。
  開始は 1 周の中で標本ごとにずらし、窓から後の最小 SOC が最も低い開始を最悪とする。ずらす 1 周は解析期間の終わりから
  「周期 + 最長継続時間 + 周期」前から始まる 1 周（前で定常に近づき、後ろで 1 周の回復を見る。足りなければ期間の始め）。
  結果はケースの `by_max_duration`: `max_dod`（解析期間全体の最小 SOC から。判定に使う）・`max_dod_from_start`（窓から後だけ）・
  `best_max_dod`（最良の開始）・`base_max_dod`（基本のモードだけ）・`worst_start`・`worst_start_in_orbit`・
  `eclipse_overlap`・`orbit_average_balance` と `energy_margin_per_orbit`（窓の立ち上がりから 1 周。1 周に 1 回入れたとき）。
  窓は標本の刻みで丸め、窓の中の標本に負荷とそのモードの姿勢則の発電を使う。
- **モードの並び**（`sequence`）: `[[sequence]]` を繰り返して解析期間を覆い、解析の開始からの経過を 1 周期で割った余りが
  入る区間のモードの負荷（日照・日陰、蝕の上書きは `mode_load` の評価のまま）と姿勢則の発電で積分する。結果は出力の
  `sequence.balance`（ケースと同じ形。`load_sunlit`・`load_eclipse` は日照中・日陰中の標本の平均）で、運用を通しての
  DoD・最小 SOC・周回ごとの収支（`orbits`）と判定を持つ。並びが無ければ `sequence = null`。

duty（状態の時間割合）はどの運用でも平均の消費電力にして一定にかける（周回内の時間分布は扱わない）。使うモードに
duty があればケースと並びの `duty = averaged` で示す。KPI の最大 DoD・軌道平均の収支（最悪）は、各ケースの判定に使った
運用の値（`by_max_duration` があればそちら）と並びの値から選ぶ。

```toml
# analysis/power.toml（並びと基本のモード）
base_mode = "NOMINAL"   # 省略時は並びの最初の区間のモード

[[sequence]]
mode     = "NOMINAL"
duration = { value = 76.0, unit = "min" }

[[sequence]]
mode     = "DOWNLINK"
duration = { value = 9.6, unit = "min" }
```

解析ケースの `mode` が指定されているときは、そのモードの `[law.<mode>]` が必要で、別のモードの姿勢則では代用しない。
モードが未指定なら、姿勢則が 1 件だけのときに限り使う。0 件または複数なら不足入力として停止する。

### 5.2 熱解析の条件

`thermal.transient` は解析ケースの `thermal_case` で環境条件を選ぶ。省略時は熱条件が 1 件だけなら使い、
複数なら解析ケースの `thermal_case` を不足入力として示す。選択した ID が存在しない場合も不足入力とする。
解析ケース自体が未選択の場合は `analysis.thermal.case` を示す。条件が 0 件でも停止する。
`thermal.steady` は引き続きすべての熱条件を解析する。
両解析とも、熱条件の `beta` がある場合だけ解析ケースの β を上書きし、省略時はケースの β を保持する。
軌道の中心天体が地球以外なら、地球の形態係数・アルベド・赤外放射を用いる熱環境は未対応のため入力エラーで停止する。

### 5.3 姿勢解析の機体外形

擾乱に使う機体外形は、`size` があり取付親のない部品が 1 個だけならその部品を使う。
複数なら候補部品 ID を不足入力の文に示して停止し、ID 順には選ばない。0 個の場合の扱いは従来どおりとする。

### 5.4 共通解析設定

系の一覧は `AnalysisSystem` が正本で、API の `DefsView.systems` に同じ順で返す。`aocs` の表示名は日英とも AOCS、`payload` は日本語「ミッション機器」・英語「Payload」。Payload は解析を持たず、設定項目もまだない。古い系名 `attitude` は受け付けない。

系に属さない設定は `analysis/common.toml` に置く。プロジェクトの設定一覧では scope は `analysis`、system は null。
`default_case` は解析ケース ID の文字列。§5.1 の対象解析と電力収支の上段の姿勢則にだけ使い、省略時はケース未選択の扱いを維持する。
CLI と画面は同じ `SetSetting` を使う。例: `fune setting set analysis.common.default_case hot_imaging`。

```toml
# analysis/common.toml
default_case = "hot_imaging"
```

### 5.5 電源の逆算サイジング（`power.sizing`）

電源の系に属する定常解析。SAP と電池の必要量を出力し、部品の属性への書き戻しはしない。
入力は軌道・`conditions.orbit.duration`・電力を持つ部品とモード・`solar_panel`（法線つき）と `battery` の部品・姿勢則・電源設定。
部品・軌道・期間・モード・姿勢則・`analysis.power.dod_max` が足りなければ不足入力を返す。
明示した解析ケースにモードがあればそのモード、無ければ全モードをそれぞれ逆算し、
必要面積が最大のモード（`area_mode`）と必要容量が最大のモード（`capacity_mode`）で決める。
成長余裕・員数・冗長・蝕の上書きは `power.mode_load` の集計を共有する。
割り当てや電力値の不足を含むモードでは要求を過小評価しないよう解析を止める。

最長蝕は `orbit.eclipse_season` を同じ軌道・期間・季節刻みで呼ぶ（本影と半影の合計）。
日照時間は軌道周期から最長蝕を引き、寿命はミッション期間を Julian year に換算する。
太陽距離は 1 AU に限り、太陽定数は sat-astro の値を使う。1 AU 以外は未対応の入力エラー。
地球の太陽距離の季節変動はこの近似では扱わない。

`Pd`・`Pe` を日照・蝕の負荷、`Td`・`Te` をその時間とすると、必要発電は
`(Pe*Te/Xe + Pd*Td/Xd)/Td`。必要 SAP 面積はこれを EOL の単位面積あたり発電で割る。
発電は既存の `solar` モデルで計算し、`cell_efficiency`・`packing_factor`・`eol_factor` を部品ごとに使う。
入射は `power.energy_balance` と同じ時系列・SAP・モードごとの姿勢則（`analysis.aocs.law.<モード>`、無ければ上段の姿勢則）で
`power.orbit_generation` を回し、日照中の平均発電を全パネルに真上から当たるときの発電で割った入射係数として掛ける
（太陽光の強さは 1 AU に固定し、変換効率は Xd で数えるので時系列には入れない）。
さらに `(1-annual_degradation)^lifetime_years`・`inherent_degradation` を掛ける。
SMAD の式は日照中の発電を平均で扱うので、日照中に発電が負荷を下回る姿勢（地心指向の固定パネルなど）では
日照中にも電池から出す分を数えず、面積と容量を過小評価する。その場合は逆算した値を `power.energy_balance` で確かめる。
異なるパネルが混在するときは、現在の面積配分を保って拡大縮小する設計を仮定する。
経年・固有の劣化を既存の `eol_factor` に含めた設計では、追加の劣化を重ねないよう上書きする。

1 組の必要電池容量は `Pe*Te/(dod_max*N*Xbattery)`、合計容量はその N 倍。
既存の集中電池モデルと同様に設計容量は全部品の容量の合計、DoD と放電効率は先頭の電池から取る。
面積と容量は部品の員数を含む。余裕は `spec::ratio_margin`、判定は `ratio_margin_verdict`（警告幅 0）。
負荷や蝕が 0 で要求が 0 なら余裕は null（判定 OK）。

出力 `SizingOutput` は必要発電・面積・電池容量（合計と1組）・設計面積/容量・両余裕・決めたモード・モードごとの逆算（入射係数を含む）・日照/蝕の負荷と時間・
最長蝕の標本（日付付き）・使用した仮定 `used`・判定を持つ。電源ページの KPI は「SAP の余裕」「バッテリーの余裕」。
任意の `cell_voltage`・`cell_current` と `sizing.bus_voltage` が揃えば、直列数はバス電圧/セル電圧、
並列数は必要電流/EOL のセル電流をそれぞれ切り上げる。異種セルの混在や属性不足では直並列を出さない。
電圧の温度変化・劣化は扱わず、電流に EOL・経年・固有劣化と面積を決めたモードの入射係数を適用する。

上書きは `analysis/power.toml` の `[sizing]`。すべて任意で、既定値は解析が持つ。

| 設定 | 省略時 |
|---|---|
| `sunlit_efficiency`（Xd） | `analysis.power.conversion_efficiency` があればその値、無ければ 0.8 |
| `eclipse_efficiency`（Xe） | 0.6（SAP から蝕の負荷までの経路全体） |
| `battery_efficiency`（Xbattery） | 先頭の電池の `discharge_efficiency`（その省略時は既存モデルの 1） |
| `annual_degradation` | 0（追加の経年劣化なし） |
| `inherent_degradation` | 1（追加の固有劣化なし） |
| `battery_strings`（N） | 1 |
| `bus_voltage` | なし（直並列を出さない） |
| `sun_distance` | 1 AU |

部品の `packing_factor`・`eol_factor` の省略時は既存の発電モデルと同じく 1。
DoD・セル効率・面積・電池容量にサイジング固有の既定値は設けない。

```toml
# analysis/power.toml
dod_max = 0.3

[sizing]
sunlit_efficiency = 0.8
eclipse_efficiency = 0.6
annual_degradation = 0.02
bus_voltage = { value = 28.0, unit = "V" }
```

## 6. 製品ツリーと取付

### 6.1 2 つの階層

部品には「どの系に属するか（機能）」と「どこに付いているか（物理）」の 2 つの親がある。
RW は AOCS に属するが、物理的には -Z パネルに付いている。1 つの木で両方を表そうとすると、どちらかが歪む。

- **製品ツリー（機能）** = `parts/` のディレクトリ構成。配分との比較とバジェットの集計に使う。
- **取付の階層（物理）** = 各部品の `mount`（取付先の部品と取付点）。3D 配置、重心・慣性の計算、熱の接触に使う。
- 位置は取付点に対する相対値だけを書く。パネルを動かせば、付いている部品も一緒に動く。機体座標は Resolved で導出する。

### 6.2 ディレクトリの規則

規則は 1 つだけ。**ディレクトリだけならまとまり（Group）、同じ名前のファイルがあればそれ自身も部品（物理的な組立品）**。

```
parts/
├── aocs/              機能のまとまり: ディレクトリだけ（ファイル不要）
│   ├── RW_X.toml
│   └── STT.toml
└── power/
    ├── BAT.toml
    ├── SAP_PX.toml    物理的な組立品: 普通の部品ファイル（質量・取付・取付点）
    └── SAP_PX/        同じ名前のディレクトリ = SAP_PX の子部品
        ├── CELLS.toml
        └── HINGE.toml
```

- Group の ID はディレクトリ名。部品の ID と同じ名前空間で一意にする。
- Group は質量も取付も持たない。最上位のディレクトリ名は `power`・`aocs`・`cdh`・`comm`・`thermal`・`structure`・`payload`。それ以外は `fune check` の error（使える ID を列挙）。入れ子の組立品・フォルダの ID は任意。
- 部品の系は `Design::system_of` だけが決める。置き場所の最上位の系 → 種別の役割の系（`role_system`、継承を含む）→ `None`。`parts/` 直下の部品も許し、役割があれば系に属する。
- 系の表示名は組み込みの語彙 `labels.section`（日英）。プロジェクトによる上書きはしない。API の `PartRow.system` は `System | null`、ツリーとバジェットの行も `system` と日英の表示名を返す。Web の色はこの系を `lib/systems.ts` の唯一の対応で引く（未所属は中立色）。配分は `budget/allocation.toml` に書く。
- 物理的な組立品（`SAP_PX`）は自分の質量・取付先・取付点を持ち、子部品は `mount = { on = "SAP_PX", … }` で組立品に付く。
  バジェットでは「組立品自身＋子部品の合計」がその組立品の質量になる。
- 部品を別のまとまりへ移すとファイルの移動になり、git は rename として表示する。
- `CODEOWNERS` でディレクトリごとにレビュー担当を決められる。

## 7. modes / nets / harness

### 7.1 置き場所の原則

- **1 つの部品だけで完結する情報**（質量、ポートの所属先など）は部品ファイルに書く。
- **複数の部品にまたがり、それ自体が名前や属性を持つもの**（モード、ネット、ハーネス）は別ディレクトリにする。
- **参照は ID で行う。** 部品の ID は全体で一意なので、どのディレクトリからでも `RW_X` で指せる。
- `nets/` と `harness/` は平らに並べる。ディレクトリが「親」の意味を持つのは `parts/` だけにする。

```
my-sat-design/
├── parts/      部品そのもの        ports で nets を指す
├── modes/      運用モード          → parts を参照（部品 → 状態）
├── nets/       電気的なつながり     ← parts の ports から参照される
├── harness/    ケーブル区間（物理）  → parts のポートと nets を参照
└── cases/      解析ケース          → modes を参照（モードの時系列）
```

### 7.2 modes/: 1 モード 1 ファイル、「部品 → 状態」の表

表示順は `fune.toml` の `[modes] order` に置く。一覧のモードを指定順、未指定のモードを後ろに ID 順で並べる。
省略時は ID 順。存在しない ID は `check` の警告となる。正規形では実際の順が ID 順と同じなら `[modes]` を省く
（存在しない ID がある一覧は警告を残すため保持する）。名前の変更は一覧も書き換え、削除は一覧から除く。追加は一覧があるとき末尾に足す。

`fune mode move <ID> --before <ID> | --after <ID> | --first | --last` で移動する。
GUI の列見出しの「左へ移動」「右へ移動」も `POST /ops/from_command` の同じ service 関数を使う。
表示順の正本は `Design::ordered_modes`。モード表・CLI の一覧・解析のモード列はこの順を使い、Web は並べ直さない。


```toml
# modes/NOMINAL.toml
label       = "定常"
description = "地心指向で待機。通信は可視時のみ"

[states]
BAT    = { state = "on" }
HEATER = { state = "off" }
OBC    = { state = "on" }
PCDU   = { state = "on" }
RW_X   = { state = "on" }
RW_Y   = { state = "on" }
RW_Z   = { state = "on" }
STT    = { state = "on" }
TRX    = { state = "rx", duty = { tx = 0.1 } }
```

- `state` はカタログ（kind）が定義する機器の状態から選ぶ。電力の値はカタログ側にあり、ここには書かない。
- `duty = { tx = 0.1 }` は「周回の 10% は tx、残りは基本の状態（rx）」の意味。
- モードの消費電力は「各部品の状態ごとの電力 × 時間割合」の合計として導出する。GUI では「部品 × モード」の表で編集する。
- 表に書かれていない部品は 0 W にせず、「未指定」の警告を出す。

部品の冗長とモードの稼働台数は次のように書く。

```toml
# parts/aocs/RW.toml（ほかの部品の欄は省略）
count = 4
redundancy = "warm"
```

```toml
# modes/NOMINAL.toml
max_duration = { value = 90.0, unit = "min" }

[states]
RW = { state = "on", active = 3, spare_state = "idle" }
HTR = { state = "off", duty = { on = 0.2 } }

[eclipse]
HTR = { state = "on" }
```

- `active` は `count` 以下。省略は全台。明示するには `redundancy = "cold"` または `"warm"` が必要。
- 稼働台は `state` と `duty` に従い、残りの台は常時予備の状態にいる。cold の予備は組み込みの停止状態 `STATE_OFF`。
  warm は `spare_state`、種別の語彙の `standby`、`idle` の順で選ぶ。予備状態が決まらない場合、語彙に無い場合は error。
- `[eclipse]` は、書いた部品の割り当て全体を蝕時だけ置換する（`active`・`duty` などの欄を個別に継承しない）。
  書かなかった部品は `[states]` を使う。`[states]` に無い上書き先は未指定と同様に warning とし、解析には足さない。
- `max_duration` は時間の次元を検査し、CLI の一覧と GUI のモード見出しに表示する。`power.energy_balance` は、このモードを
  基本のモードの上で最長継続時間だけ続けたときで判定する（§5.1）。解析期間やモードの時間割合（`mode_share`）には影響しない。
- 台数の評価は `sat_analysis::modes::counted_states` だけ。電力は「稼働台 × 状態の平均電力 + 予備台 × 予備状態の電力」。
  `power.mode_load.total` は日照時、`total_eclipse` は蝕時（成長込みはそれぞれ `*_with_growth`）。
  `power.energy_balance` はこの日照時・蝕時の負荷を使う。C&DH は日照時の割り当てで同じ台数を数える。
- 熱のモード負荷も同じ台数評価を使う。現段階は日照時の割り当てだけを使い、蝕の上書きによる発熱の変化と軌道平均は扱わない。
- CLI は `fune set RW.redundancy warm`、`fune mode set NOMINAL RW on --active 3 --spare-state idle`、
  `fune mode set NOMINAL HTR on --eclipse`。`fune mode unset NOMINAL HTR --eclipse` で日照時の割り当ての継承へ戻す。
  `fune mode duration NOMINAL --max-duration "90 min"` で継続時間を書き、旗を省くと消す。
  HTTP は既存の `/ops/from_text` と `/ops/from_command` を通して Op にする。
- GUI はモード表のセルの編集欄で稼働台数・予備状態を変更し、「蝕の状態」の別欄で上書きを保存する。
  部品詳細の基本欄で冗長方式を選び、モードの見出しで最長継続時間を編集する。
### 7.3 nets/: 1 ネット 1 ファイル、ネット自身の性質だけ

```toml
# nets/P12V_AOCS.toml
kind   = "power"
label  = "AOCS 12 V 系"
source = "PCDU.OUT3"
rail   = { value = 12.0, unit = "V" }
limit  = { value = 2.0, unit = "A" }
```

```toml
# nets/CAN_BUS.toml
kind     = "data"
label    = "システム CAN"
protocol = "CAN"
bitrate  = { value = 1.0, unit = "Mbit/s" }
master   = "OBC.CAN1"
```

どの部品がつながっているかはネット側に書かず、部品側のポートに書く。部品を足しても共有のファイルを書き換えないので、衝突が起きない。
系統図は「ネット＋各部品のポート」から組み立てて描く。

- `source`・`master`（とハーネスの `from`・`to`）は `"部品.ポート"` の文字列で書き、読み込むときに `PortRef`（部品 ID とポート名）にする。
  この形でなければ、そのファイルの形の誤り（`toml.schema`、行付き）としてファイルを読まない。部品やポートが無いことは `fune check` が検査する。
- **戻り（GND）のネット**は、`kind = "power"` で `rail` が 0 のもの、または ID に `GND`・`RTN` の語（`_` で区切った語。大文字小文字を
  無視）を含むもの（`fune_model::design::Net::is_return`）。系統図では GND の層に描く。表示名（`label`）は見ない。

```toml
# parts/aocs/RW_X.toml（抜粋）
catalog = "lab/rw_18mNms"

[ports.PWR]
net = "P12V_AOCS"

[ports.DATA]
net = "CAN_BUS"
```

### 7.4 harness/: 1 区間 1 ファイル、物理的なケーブル

```toml
# harness/H_PCDU_RWX.toml
label = "PCDU → RW_X 電源"
from  = "PCDU.OUT3"
to    = "RW_X.PWR"
nets  = ["P12V_AOCS"]
wire  = "lab/awg22_ptfe"
cores = 2

[length]
value       = 0.35
unit        = "m"
from.source = "layout_estimate"
from.note   = "3D 配置からの概算"
```

- ネットが「論理的なつながり」なのに対して、ハーネスは「実際の線」。1 本の束に複数のネットを通せる。
- 質量（長さ × 線材の単位長さ質量 × 芯数）と電圧降下（電流 × 抵抗）を区間ごとに計算する。電流はモードの消費電力とレール電圧から出す。
- 概念設計の初期にはハーネスのファイルを書かなくてよい。その間は `budget/policy.toml` の見込み割合で代用する。

### 7.5 よくある操作で変わるファイル

| 操作 | 変わるファイル |
|---|---|
| 部品を追加（RW_Z） | `parts/aocs/RW_Z.toml`（新規）と、各 `modes/*.toml` に 1 行ずつ |
| モードを追加 | `modes/NEW.toml`（新規）だけ |
| 部品をネットにつなぐ | その部品ファイルの `ports` に 1 行 |
| ネットを新しく作る | `nets/X.toml`（新規）と、つなぐ部品の `ports` |
| ハーネス区間を足す | `harness/X.toml`（新規）だけ |
| 部品の名前を変える | 部品ファイルの移動と、参照している modes・harness・nets の `source` の書き換え（Rename の Op が一括で行う） |
| 配分を付け替える | `budget/allocation.toml` だけ |

### 7.6 モードを「モードの側」に置くトレードオフ

部品を 1 つ足すと、モードの数だけファイルが 1 行ずつ変わる。逆に、部品ファイルに `[modes] NOMINAL = "on"` と書く方式にすると、
モードを 1 つ足すたびに全部品のファイルが変わり、1 つのモードの全体像も 1 ファイルで見られなくなる。
「電力の担当者が 1 ファイルでモードの全体を確認できること」を優先して、モードの側に置く。
行は ID 順に並べるので、隣り合う ID を 2 本のブランチで同時に足さない限り、git は自動でマージできる。

## 8. 配分とマージン方針（budget/）

配分はサブシステムのディレクトリに分散させず、1 つのファイルにまとめる。

- 配分を決めるのはシステム担当、`parts/aocs/` を触るのは AOCS 担当で、持ち主が違う。`CODEOWNERS` でレビュー担当を分けられる。
- 配分は一緒に動く。「AOCS に 0.2 kg 回して PAYLOAD を減らす」という変更が、ファイル 1 つ・差分 2 行で済む。
- 合計の確認が 1 ファイルで済む。`check()` は「配分の合計 ≤ 上限 × (1 − システムマージン)」を検査し、超えたら warning を出す。

```toml
# budget/allocation.toml
[mass]
limit         = { value = 12.0, unit = "kg" }   # ロケット側の上限
system_margin = 0.10

[mass.alloc]
aocs      = { value = 2.0, unit = "kg" }
cdh       = { value = 0.8, unit = "kg" }
comm       = { value = 1.0, unit = "kg" }
power       = { value = 3.2, unit = "kg" }
payload   = { value = 1.5, unit = "kg" }
structure = { value = 2.3, unit = "kg" }

[power]
limit         = { value = 25.0, unit = "W" }    # 軌道平均の発電見込み
system_margin = 0.15

[power.alloc]
aocs    = { value = 8.0, unit = "W" }
cdh     = { value = 2.5, unit = "W" }
comm     = { value = 4.0, unit = "W" }
power     = { value = 1.5, unit = "W" }
payload = { value = 5.0, unit = "W" }
```

```toml
# budget/policy.toml
[growth]                 # 成熟度ごとの成長余裕（質量・電力に掛ける）
off_the_shelf = 0.05
modified      = 0.10
new           = 0.20

[uncertainty]            # 不確かさを書いていない値に当てる既定の幅（±）
off_the_shelf = 0.03
modified      = 0.10
new           = 0.20

[harness]
allowance = 0.05         # harness/ が無い間、サブシステム質量に対して見込む割合
```

配分のキーは系の ID か部品の ID。系のディレクトリがまだ無くても配分できる。組立品（`SAP_PX`）のような部品にも配分できるが、任意のフォルダの ID は受け付けない。
集計は明示配分のある部品・最も近い祖先の組立品を優先し、それ以外は部品の系にまとめる（役割から決まる系も含む）。未所属は「その他」の行。種類ごとの配分だけを使い、別種類の配分先には影響されない。
`limit` は省略できる。未設定の種類は `fune check` が warning「上限が未設定」を出し、上限に対する判定を行わず現状の合計を返す。`fune init` は質量上限を決めず、SAFE モードにも表示名を埋め込まない。

## 9. Derived の出力先

Derived は「保存しない」が基本。出力先は用途で分ける。

| 出力 | 場所 | git で管理するか | 用途 |
|---|---|---|---|
| バジェット・質量特性など、すぐ計算できるもの | なし（メモリ上） | — | GUI・CLI の表示。毎回計算し直す |
| 解析・SIM の実行結果 | `.fune/runs/<解析>/<ケース>/<fingerprint>/` | しない | キャッシュと、手での再実行 |
| 残したい結果 | `results/<名前>/` | する | `fune results pin` で選んだものだけ。レビューや審査で見せる |
| 他ツールへの書き出し | `exports/`（budget.json など） | する | 外部の検証ツールなどが読むファイル |
| 審査用のまとめ（PDF など） | GitHub Release の添付 | リポジトリには入れない | `git tag pdr-2026-12` と `fune report` の出力 |

```
results/thermal-hot/
├── manifest.toml        解析 ID・ケース・入力の fingerprint・元の commit・ツールの版・pin した人と日時
├── result.json          スカラー値と系列の宣言（fune-tool/1）
└── series/temperature.csv
```

```toml
# results/thermal-hot/manifest.toml
analysis    = "thermal.case"
version     = 1
case        = "hot"
fingerprint = "sha256:9f3c2a1e"
source_rev  = "a1b2c3d"
tool        = "git:example-org/thermal-tool@1e9af19"
pinned_by   = "tanaka"
pinned_at   = 2026-11-06T10:30:00+09:00
```

計算結果を全部はコミットしない理由は次のとおり。

- 入力はきれいにマージできても結果ファイルが衝突する（2 本のブランチがそれぞれ部品を足すと、質量の合計が両方で変わる）。
- 入力と食い違った古い結果が残る。
- 差分が数値の変化で埋まり、レビューしにくくなる。

PR では CI が両方のリビジョンで計算し直した変化量をコメントするので、結果をコミットしなくても差分は見られる。
時間のかかる熱解析や、CI で動かせない外部ツールの結果だけを `results/` に pin する。pin した結果は入力の fingerprint を持つので、
入力が変われば `fune check` が「この結果は古い」と警告する。大きな系列ファイルは Git LFS に置く。

## 10. ファイル形式

### 10.1 形式の比較

| 形式 | git の差分・マージ | 手編集 | GUI からの書式保存書き込み（Rust） | 入れ子・ツリー | スキーマ・エディタ支援 | 判定 |
|---|---|---|---|---|---|---|
| **TOML** | ◎ 1 行 1 値 | ○ | ◎ toml_edit 0.25（cargo が使用） | △ 深い入れ子は読みにくい | ○ JSON Schema（Tombi / Taplo、`#:schema`） | **採用**。ツリーをディレクトリで表せば弱点に当たらない |
| KDL v2 | ◎ | ○ | ◎ kdl 6.7（書式保存） | ◎ ノードの木がそのまま製品ツリーになる | △ JSON Schema が使えず、エディタ支援も少ない。Python 側の読み手が育っていない | 次点。入れ子でツリーを書きたくなったら再検討 |
| YAML | ○ | ○（インデントや型推論の罠） | ✗ serde_yaml は deprecated、書式保存の書き込み手段がない | ◎ | ○ | 不採用 |
| JSON | △ カンマと括弧で差分が汚れる | ✗ コメント不可 | ○ | ◎ | ◎ | 機械向けの交換・書き出し専用 |
| 独自 DSL（SysML v2 風） | ◎ | ◎ | ✗ パーサー・整形・LSP・書式保存をすべて自作 | ◎ | ✗ 自作 | 不採用。将来の書き出し先としてのみ |
| CSV | ○ 行単位 | ○ Excel で開ける | ○ | ✗ | △ | 表データ専用 |
| SQLite / Dolt | ✗ / △ | ✗ | — | — | — | 不採用（理由は [architecture.md](architecture.md) §2.2） |

### 10.2 TOML プロファイル

`fune fmt` と `fune check` で強制する書き方の規則。

1. 1 ファイル 1 エンティティ。ID はファイル名の stem で、全体で一意（大文字小文字を無視、Windows の予約名は不可、`_` で始めない）。
   属性 ID には部品・カタログ品のフィールド名（`label`・`count`・`maturity` など。`fune_model::field_names::{PartFieldName, CatalogFieldName}`）を使わない
   （パスの記法 `RW_X.label` で属性とフィールドを区別できないため。`fune check` の `def.reserved`、`fune attr add` も拒む）。
2. 入れ子は 2 段まで。ツリーはディレクトリ、エンティティ間の関係は ID の参照で表す。
   例外は出典の付いた値の行（3）のドット付きキー（`[attr.<名前>]` の下の `from.*`、状態ごとの値のセルの `<状態>.*`）だけ。
3. インライン表は `{ value, unit }`、短いベクトル（`{ x, y, z, unit }`）、範囲（`{ min, max, unit }`）、1 行で収まる参照だけ。
   状態や不確かさを持つ値は `[attr.<名前>]` の下に 1 行 1 セル。
   **出典（`from`）の付いた値は 1 行 1 要素で書く。** 出典の無い値は 1 行のインライン表のまま。
   切り替えは行の長さではなく出典の有無だけで決める（`note` を少し書き足しただけで形が変わり、差分が荒れるのを避ける）。
   - 属性（`[attr]` の行）とファイルの最上位の値（ハーネスの `length` など）は、値の見出しの表（`[attr.<名前>]`・`[length]`）の下に
     `value`・`unit`（範囲は `min`・`max`、ベクトルは `x`・`y`・`z`）を 1 行ずつ、出典はドット付きキー `from.source`・`from.note` で書く。
   - 状態ごとの値のセルに出典が付くときは、`[attr.<名前>]` の下にセルの名前のドット付きキー（`on.value`・`on.unit`・`on.from.source`）で書く
     （`[attr.power.on]` は 3 段になるので使わない）。出典の無いセルは今の 1 行のまま、同じ表に並ぶ。
   - 見出しの表は親の表の値の行の後ろに置く（TOML の決まり）。親の表に値の行が残らなければ親の見出し（`[attr]`）は書かない。
   - 表の `=` は、ドット付きキーの行も含めて表ごとに揃える。
   - 読み込みはどちらの形（古い 1 行のインライン表、見出しの表・ドット付きキー）も同じ値になる。`fune fmt` が今の形に直す。

   ```toml
   [attr]
   specific_heat = { value = 1100.0, unit = "J/(kg*K)" }

   [attr.mass]
   value       = 115.0
   unit        = "g"
   from.source = "ds_example"
   from.note   = "Technical Specifications table, Mass [g]: 115."

   [attr.power]
   off            = { value = 0.0, unit = "W" }
   on.value       = 1.0
   on.unit        = "W"
   on.from.source = "sys_judgement"
   ```
4. 物理量の数値は常に float で書く（`0` ではなく `0.0`）。整数は ID・員数（`count`・`active`）・回数だけ。
5. 書き出しは TOML 1.0 の範囲（§10.3）。
6. 曲線・特性表・時系列は CSV に置き、列の単位は対になる TOML で宣言する。
7. 計算結果は書かない（§9）。
8. 先頭に `#:schema` を置き、VS Code（Even Better TOML / Tombi）で補完と検証を効かせる。

### 10.3 TOML の版

TOML 1.1.0 は 2025 年 12 月に正式化されたが、**書き出しは 1.0 の範囲に留める**。1.1 を使わないと決めたのではなく、読み手が揃うまで待つ。

**理由 1: 読み手がまだ揃っていない。** 設計データはアプリ以外（外部の検証ツール、Python のスクリプトやノートブック、エディタ）も読む。
1 つでも 1.0 にしか対応しない読み手があれば、1.1 の構文を書いた時点でその読み手では読めなくなる。

| 読み手 | TOML 1.1 | 確認方法（2026-11-06） |
|---|---|---|
| Rust（`toml` 1.2.0、`toml_edit` 0.25.18） | 対応 | crate の版表記 `+spec-1.1.0` |
| Python 3.15 の `tomllib` | 対応（3.15 で追加） | Python のドキュメント |
| Python 3.14 以前の `tomllib` | **非対応** | Python 3.14.4 で、複数行のインライン表がエラーになることを確認 |
| `tomli` | 2.4.0 から対応 | CHANGELOG |
| `tomlkit` | 0.15.0 から読める。0.15.2 は 1.0 のパーサーでも読めるよう、書き出しを 1.0 の範囲にしている | CHANGELOG |
| Taplo（VS Code の Even Better TOML が使用） | おそらく非対応（最新の 0.10.0 は 2025-05 公開で、1.1 の正式化より前） | crates.io の版 |

Python で読む道具は 3.14 以前の `tomllib` で動いていることが多く、3.14 の `tomllib` は 1.0 しか読めない。

**理由 2: 今回の書き方では、1.1 で得られるものが小さい。**

| 1.1 の新機能 | 今回のファイル形式での効き目 |
|---|---|
| インライン表を複数行に分けられる（末尾のカンマも可） | 1.0 でもサブテーブルにすれば 1 行 1 項目で書けるので、差分の読みやすさは変わらない。同じ内容を 2 通りに書けると整形の正規形がぶれる |
| `\e`・`\xHH` のエスケープ | 設計データでは使わない |
| 日時の秒を省略できる | 省略できなくても困らない |

**運用:**

- 書くとき: アプリと `fune fmt` は 1.0 の範囲だけで出力する。
- 読むとき: アプリは 1.1 も読めるが、手で 1.1 の構文を書くと Python 3.14 の読み手では読めなくなる。`fune check` が警告し、`fune fmt` が 1.0 の書き方に直す。
- 切り替える時期: Python で読む道具の実行環境（CI のイメージを含む）が 3.15 以上に揃い、エディタも 1.1 に対応したら。
  `fune.toml` の `[project] toml = "1.0"` を `"1.1"` に変えれば出力を切り替えられるようにしておく。

### 10.4 系統図の表示データ

`view/electrical.toml` は解析に影響しない。`[nodes]` は部品・グループの ID から、親からの相対の整数座標
（`x`, `y`, 任意の `w`, `h`）への表。`[edge]` は手で引いた線の経路で、線を指すキーから `ViewEdge` への表。
`points` は画面の絶対座標の整数 `[x, y]` の配列（2 点以上）で、隣り合う点は水平か垂直に並ぶ。
`source_side`・`target_side`（省略可、`left` / `right` / `top` / `bottom`）は端子をカードのどの側に出すか、
`source_offset`・`target_offset`（省略可、整数 [px]）はその側の上の位置（左か上の角からの距離。省略なら端子の
並びで決まる位置）を表す。どれも表示だけを変え、部品・ネット・解析の入力は変わらない。座標と位置は整数のまま書く
（`fune fmt` も小数にしない）。GUI では線を選んで両端の丸をドラッグすると、その端をカードの縁の好きな位置へ付け替えられる。
手で引いた経路の端のカードをあとから動かしたときは、描くときにその端から側の外へ短い足を出して経路の続きへつなぎ直す
（保存した経路は変えない）。

```toml
[nodes]
power = { x = 20, y = 20 }

[edge]
"P12V:PCDU.OUT3>RW_X.PWR" = { points = [[100, 80], [400, 80], [400, 200]], target_side = "top", target_offset = 40 }
```

線を指すキーは `ネット:始点の端子>終点の端子`。端子は `部品.ポート`、ジャンクション（中心の無いネットの分岐）は
`(ネット)`、同じ 2 枚のカードの間をまとめた線はネットを `+` でつなぐ（`P5V+P12V:…`）。向きは図の線の向き
（電源は供給元から、データはマスターから。供給元の書かれていない電源の母線は配電へ向かう）。
キーはネットと端子だけで決まり、自動配置の順番や座標を含まないので、並べ直しても同じ線を指す。
同種をまとめたカードの端子は、まとめた部品のうち ID がいちばん小さいものの名前を使う。

保存した位置と経路は自動配置より優先する。`SetViewEdge` / `UnsetViewEdge` は 1 本の経路を書く・消す、
`ResetView` はその図の `[nodes]` と `[edge]` を両方消す。CLI のコマンドは
`fune view edge-set electrical <キー> --points "x,y x,y …" [--source-side …] [--target-side …] [--source-offset …] [--target-offset …]`、
`fune view edge-reset electrical <キー>`、`fune view reset electrical`。

自動配置の条件は次のとおり（`web/src/features/diagram/layout/`。部品の ID・名前・役割は見ない）。

- 枠の見出しは表示の言語の名前だけ（系は API の組み込みの語彙の日英名、組立品や入れ子は設計の表示名。無ければ ID）。
- 列や層は決めない（電源の枠を左端に置く、といった決まりも置かない）。いちばん外の枠（サブシステム）を縦横の格子の
  升目に、カードを枠の中の格子に置く。枠の中のすき間は横 56・縦 48 px で、線が通れる幅にする。枠の中をカードの
  縦 1 列（行 > 列 + 1）や横 1 行（列 > 2 × 行 + 1）に並べる形は使わない（図全体が列に見えるため）。
- 位置は、表示している層にかかわらず全部の層（給電・信号・GND）の線で決める。層を切り替えても枠・カード・端子の
  位置は変わらず、表示する層の線だけを引き直す。
- 置き方は探索で決める。線でよくつながる枠から順に、見積もりがいちばん小さくなる升目へ置き、そのあと升目の入れ替え・
  枠の中のカードの入れ替え・枠の中の列の数の変更を、見積もりが下がらなくなるまで繰り返す。見積もりは
  「（線の両端のカードの中心の距離（縦 + 横）の合計 + 300 px × 交差の見込み + 200 px × ほかのカードを横切る見込み）×（1 + 全体の縦横比のずれ + 0.8 × 枠の
  縦横比のずれの平均）」。式の「全体」は線の無い枠を下か右に置いたあとの形。交差の見込みは、カードの中心どうしを直線で結んだ
  ときにほかのネットの線と交わる組の数と、1 枚のカードから出る別々のネットの相手がカードの周りで入り組む数（同じ
  ネットのひと続きの数 − ネットの数）の和。ほかのカードを横切る見込みは、両端のカードの中心を縦横の 2 区間（横に出てから
  縦、縦に出てから横）で結んだとき、どちらでも両端以外のカードの上を通る線の数（横に並べたカードの奥のカードの線が
  手前のカードを回り込む並べ方を嫌う）。格子の形（1〜4 行）ごとに 1 つずつ候補を作る（`layout-estimate.ts`）。
- 詰め: 升目に置いた枠を、横・縦の順に線の相手のほうへ寄せる（相手と揃う位置の中央値）。同じ帯にある枠とは並びの
  順を守り、64 px 以上のすき間を空ける。升目の大きさで空いていた余分なすき間と線の長さが縮み、枠どうしは近づきすぎない。
- 端子の上下左右の側は、線の相手（の平均）がカードの外のどちらにいちばん離れているかで選ぶ。同じ側の端子は相手の
  位置の順に並べ（左右の側は 24 px、上下の側は端子の名前が重ならない間隔）、端子の数に合わせてカードを大きくする。
  何本も線が出る端子（バス）で、相手が端子の側の反対側（カードの裏）にいる線は、相手のいる側に分かれ口を作って
  そこから出す（同じ端子なので端の名前も同じ。カードを大回りして裏の端子へ戻る線を作らない）。分かれ口も端子と同じく
  側の上に並べ、手で端を付け替えた線では手で決めた位置を優先する。
- 線は 8 px の格子の上の直交の経路探索（A*）で引く。カードと枠の見出しの文字は通らず、カードの横を通る線は縁から
  16 px 以上離れる（端子の出口だけはその端子の線が通る）。費用は 1 格子 1 に、折れ 10、ほかのネットの線との交差 30、ほかのネットの線と平行に重なる所 400（実際には通らない）、
  ほかのネットの線のすぐ隣を平行に走る所 1.5、ほかのネットの線の上で折れる所 40、両端の入っていない枠の中 0.4、
  枠の縁（とその内外 1 格子）に沿って走る所 3。同じネットの線の上は 0.9 で、幹を共有して枝分かれする。全部の線を
  引いたあと、1 本ずつ外して引き直す。経路の両端は端子の点（React Flow が線をつなぐ、端子の外向きの縁）。
- 見積もりの小さい 6 つの候補に（全部の層の）線を引き、「20 × 経路の見つからない線の数 + 0.5 × 線の交差の数 + 4 ×
  重なりの数 + 8 × 縦横比のずれ（比の対数の差）+ 0.4 × 折れの数 + 線の総延長 / 図の対角線」がいちばん小さいものを使う
  （同じ端子・同じネットの線どうしは交差にも重なりにも数えない）。毎回同じ結果になる。交差は少ないほうがよいが、折れと
  長さほどは重く見ない（交差には飛び越しを描くので読める）。
- 端子の選び直し: 選んだ候補の線を全部引いたうえで、端子ごとに次の案を作り、その端子の線だけを引き直した費用（ほかの
  線はそのまま）が下がるものを残す（`layout-refine.ts`）。
  - 側: ほかの 3 つの側の、同じカードのほかの端子と重ならない位置のうち線の相手にいちばん近い所から出す（空きが無ければ
    並んだ端子のすぐ外側。端子が増えるとカードは大きくなるので、それを見越す）
  - 位置: 端子を出てすぐ横へずれてまた外へ向かう線（段）を、段の分だけ端子を側の上でずらしてまっすぐにする
  案は得の大きい順に 6 つまで 1 つずつ試し、全体の費用が下がる案だけを採る。採った案があれば案を出し直し、4 回まで
  繰り返す（ある端子を動かすと別の端子が動ける空きができるため。例: OBC の SPW1 を下へ移すと LVDS1 が左へ移れる）。
  端子の側は相手の中心の方向で決めるが、実際の線は間のカードや同じネットの幹を回るので、選び直したほうが線が短く、
  折れが少なくなることがある。
- 線の無い枠（構体など）は 3 列の格子にして、図の下か右のうち全体が 16:10 に近くなる方に置く。
- 線のラベル（プロトコル名）は同じ端子から出る同じプロトコルの線に 1 つだけ付け、線の脇でカード・枠の見出し・
  ほかのラベル・線と重ならない場所に置く。
- 端子の名前はカードの外の線の脇に置く（上下の側は線の右、左右の側は線の上）。上下の側の端子は、いちばん長い名前が
  隣の線に届かない間隔で並べる（側の上の位置を選び直したときも、この間隔より近づけない）。
- 交差には飛び越し（半円）を描く。水平の線がほかのネットの垂直の線をまたぐ所に入れる。手で引いた線や、カードを
  動かしてつなぎ直した線にも入れる（各線の描いている経路を `line-store.ts` で分け合う）。
- 手で動かして保存した位置は自動配置より優先する（保存した枠の大きさが、自動配置で並べた中身より小さいときは中身に
  合わせて広げる）。「位置を自動に戻す」のボタンは保存した枠とカードの位置を消して自動で並べた位置に戻す（手で引いた線の経路は
  残し、端はカードに付いていく）。「位置と線を自動に戻す」は位置と線の経路を両方消す（`ResetView`）。
- 拡大・縮小は 10%〜1000%。左下の欄に、いまの倍率（押すと 100%）、全体を見る、選んだカード・線に寄る、全体図（ミニマップ）
  の表示があり、キーでは + / − で拡大・縮小、0 で全体、1 で 100%、F で選んだものに寄る（`DiagramZoom.tsx`）。
  マウスはホイールで上下、Shift + ホイールで左右に動かし、Ctrl / ⌘ + ホイールでカーソルの位置を中心に 1 目盛り 1.2 倍ずつ
  拡大・縮小する（トラックパッドのピンチはそのまま拡大・縮小）。
- 配置の編集（位置・線の経路・端の付け替え・上の 2 つのボタン）は、⌘Z / Ctrl+Z で元に戻し、⇧⌘Z / Ctrl+Y でやり直せる
  （ツールバーのボタンでも）。編集する前の配置から戻す Op（`SetViewLayout` / `SetViewEdge` / `UnsetViewEdge`）を作って
  覚えておき、戻すときは普通の編集として送る。覚えるのはそのタブで行った最近の 100 回まで。
- デモの表示モデルの写し（`web/src/test/diagram-model.ts`）で、カードが重ならないこと・枠どうしが近づきすぎないこと・
  列に並ばないこと・直交で両端が端子の点であること・ほかのネットの線と重ならないこと・自分のカードの真裏を回り込まないこと・
  層を切り替えても位置が同じこと・毎回同じになることをテストする。

## 11. ディレクトリ構成（schema v2）

```
my-sat-design/
├── fune.toml                    # プロジェクト設定・共有カタログ・TOML の版
├── catalog.lock                 # 共有カタログの固定 commit
├── budget/
│   ├── allocation.toml          # 配分（システム担当）
│   └── policy.toml              # 成長余裕・不確かさの既定幅・ハーネスの見込み
├── defs/
│   ├── attribute/<id>.toml
│   └── kind/<id>.toml
├── catalog/<id>.toml            # このプロジェクト固有のカタログ
├── parts/                       # 製品ツリー = ディレクトリ
│   ├── aocs/
│   │   ├── RW_X.toml
│   │   ├── RW_Y.toml
│   │   ├── RW_Z.toml
│   │   └── STT.toml
│   ├── power/
│   │   ├── BAT.toml
│   │   ├── PCDU.toml
│   │   ├── SAP_PX.toml          # 物理的な組立品
│   │   └── SAP_PX/
│   │       ├── CELLS.toml
│   │       └── HINGE.toml
│   └── structure/
│       ├── BUS_FRAME.toml
│       └── PANEL_MZ.toml        # 取付点を持つ構体部品
├── modes/<ID>.toml
├── nets/<ID>.toml
├── harness/<ID>.toml
├── conditions/                  # 前提条件
│   ├── orbit.toml               # 軌道と軌道の話題の解析設定
│   ├── launch.toml              # 打ち上げ条件
│   └── ground/<ID>.toml         # 地上局
├── analysis/<系>.toml           # 系ごとの解析設定（話題のファイルは作らない）
├── analysis/common.toml         # 共通の解析設定
├── mission/                     # #9 用の予約ディレクトリ（今は読まない）
├── cases/<ID>.toml
├── sources/<ID>.toml
├── requirements/<ID>.toml
├── tables/<ID>.csv  tables/<ID>.toml
├── view/
├── results/<名前>/               # pin した解析結果だけ
├── exports/                     # 他ツールへの書き出し
├── .github/CODEOWNERS
└── .fune/                       # gitignore: キャッシュ、runs/
```

`CODEOWNERS` の例（担当者名は仮）を示す。

```
/budget/        @system-lead
/modes/         @system-lead @power-lead
/parts/aocs/    @aocs-lead
/parts/power/     @power-lead
```

### サンプル

`fune.toml`

```toml
[project]
name   = "my-sat"
schema = 2
toml   = "1.0"              # 書き出す TOML の版

[modes]
order = ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]  # 省略時は ID 順

[[catalog]]
name = "lab"
git  = "https://git.example.com/example-lab/parts-catalog"   # 架空の例
rev  = "v2026.11"

[[catalog]]
name = "local"
path = "catalog"
```

`parts/aocs/RW_X.toml`

```toml
#:schema ../../.fune/schema/part.json
catalog    = "lab/rw_18mNms"
candidates = ["lab/rw_18mNms", "lab/rw_30mNms"]

[mount]
on    = "PANEL_MZ"
point = "RW_SLOT_1"

[pose]
position    = { x = 0.0, y = 0.0, z = 12.0, unit = "mm" }
orientation = { w = 1.0, x = 0.0, y = 0.0, z = 0.0 }

[attr.mass]
value       = 0.62
unit        = "kg"
from.source = "em_test_2026_10"
from.note   = "EM 実測"

[ports.PWR]
net = "P12V_AOCS"

[ports.DATA]
net = "CAN_BUS"
```

`parts/structure/PANEL_MZ.toml`

```toml
kind     = "panel"
maturity = "new"

[attr.mass]
value       = 0.35
unit        = "kg"
from.source = "cad_2026_09"

[mount]
on    = "BUS_FRAME"
point = "FACE_MZ"

[mount_points.RW_SLOT_1]
position    = { x = 30.0, y = 0.0, z = 5.0, unit = "mm" }
orientation = { w = 1.0, x = 0.0, y = 0.0, z = 0.0 }
```

`parts/power/SAP_PX.toml`（物理的な組立品）

```toml
kind     = "solar_wing"
maturity = "new"

[attr.mass]
value       = 0.18
unit        = "kg"
from.source = "cad_2026_09"
from.note   = "基板と構造のみ"

[mount]
on    = "PANEL_PX"
point = "HINGE"

[pose]
position    = { x = 0.0, y = 0.0, z = 0.0, unit = "mm" }
orientation = { w = 1.0, x = 0.0, y = 0.0, z = 0.0 }

[mount_points.CELL_AREA]
position    = { x = 0.0, y = 120.0, z = 2.0, unit = "mm" }
orientation = { w = 1.0, x = 0.0, y = 0.0, z = 0.0 }
```

`parts/power/SAP_PX/CELLS.toml`（組立品の子部品）

```toml
catalog = "lab/triple_junction_cell"
count   = 24

[mount]
on    = "SAP_PX"
point = "CELL_AREA"
```

## 12. 共有カタログ

カタログは衛星をまたいで使い回す資産なので、チームで共有するリポジトリ（例: `example-org/component-catalog`）に置き、
各衛星のリポジトリからは版を固定して読み取り専用で参照する（`catalog.lock` に commit を記録）。
参照は `lab/rw_18mNms` のように出所付きで書き、同じ名前の衝突を防ぐ。
カタログの値を直したいときは、共有リポジトリに PR を出すか、その衛星の Part で上書きする（上書きは由来付きで残る）。

## 13. 未決事項

| # | 問い | 既定案 |
|---|---|---|
| M1 | 物理的な組立品を「同名のファイル＋ディレクトリ」で表す規則でよいか | よい。特別なファイル名（`_assembly.toml`）は使わない |
| M2 | 共有カタログを最初から分けるか | 最初は `catalog/` に置き、2 機目が出た時点で共有リポジトリに切り出す |
| M3 | 員数（count）と個別 Part の使い分け | 個別の姿勢・配線が要るものは個別 Part。同じ状態で動く機器の組は count、予備との台数配分は active と redundancy |
| M4 | Range と Uncertain の区別 | 動作温度範囲のような「仕様の範囲」は Range、値のばらつきは Uncertain |
| M5 | モードの表に書かれていない部品の扱い | 「未指定」の警告。kind に既定の状態は持たせない（暗黙の 0 W を作らないため） |
| M6 | TOML 1.1 に切り替える時期 | Python の実行環境が 3.15 以上に揃い、エディタが対応した時点 |

## 14. 実装との差分

実装（fune-model / fune-store / fune-ops）は本書に沿っているが、次の点が異なる。API の正確な形は各 crate の型と `///` の説明にある。

| 本書 | 実装 | 理由 |
|---|---|---|
| `Authored<Quantity>` と `Quantity { number, unit }` | `Quantity { value, unit, from }` | ファイルと API の形を揃えるため |
| `Value` の enum | `{ type, data }` のタグ付き enum | TS の型生成に合わせるため |
| エンティティを `Arc` で保持し、位置を値に持つ | 素の map で保持し、行の位置は別の `SourceMap` に持つ | 単純さのため |
| kind は継承を持たない | 組み込みの kind は基底の kind `component` を継承する | 共通属性（質量・外形・温度範囲）を一度だけ書くため |
| error の診断があると編集できない | 編集で **新しく** error が増える Op だけを拒否する | 既存の error があっても直す編集はできるようにするため |
| 取付の連鎖を model で合成する | model は連鎖を未合成のまま渡し、projection が sat-astro の Quat で合成する | fune-model が sat-astro に依存しないため |
| ID はファイル名から（ファイルに書かない） | 同じ。加えて JSON の Op ではペイロードの `id` を受け取り、ファイルに `id` があってファイル名と違えば `id.mismatch` | GUI と `fune apply` から新規作成できるようにするため |
| 属性の既定値 | 組み込みの属性に `default` を持てる（例: 比熱 900 J/(kg·K)）。既定値から解決した値は出所 `kind_default`・根拠 `placeholder` | 仮値であることを画面で示すため |

まだ読み込んでいないもの: #9 用の `mission/`、`requirements/` と `tables/`（ディレクトリは認識する）。`#:schema` の見出し行はまだ書き出していない。
