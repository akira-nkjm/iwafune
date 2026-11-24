import {
  keepPreviousData,
  type QueryClient,
  queryOptions,
  type UseMutationOptions,
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "sonner";
import { getT, localized } from "@/i18n";
import { paramPath } from "@/lib/design";
import { clearFieldErrors, setFieldErrors } from "@/lib/field-errors";
import { copyText, recordChange } from "@/lib/history";
import { useDebounced } from "@/lib/use-debounced";
import { type ApiError, api, seg } from "./client";
import { invalidatedByOps, keys } from "./keys";
import { q } from "./queries";
import type {
  ApplyRequest,
  ApplyResult,
  BudgetKind,
  BudgetView,
  CatalogItemView,
  CommandText,
  DesignView,
  Op,
  PartRow,
  PlaceBodyRequest,
  ProjectInfo,
  SetText,
  UnsetText,
} from "./types";

/** 編集の説明（トーストと変更履歴の見出し） */
export interface OpsRequest {
  ops: Op[];
  summary: string;
}

/** Op が触るパラメーターのパス（欄のエラーを消すため） */
function opPaths(ops: readonly Op[], design: DesignView | undefined): string[] {
  return ops.flatMap((o) => {
    if (o.op === "set_value" || o.op === "unset_value")
      return [
        paramPath(design?.parts.find((p) => p.id === o.part) ?? { paths: {} }, o.attr, o.state),
      ];
    if (o.op === "set_setting") return [o.path];
    return [];
  });
}

/**
 * トーストに変化を出すバジェット: すべての種類（一覧は API の `DefsView.budget_kinds`。設計をまだ読んでいなければ出さない）。
 * そのうち、編集の前に読んであったものだけを比べる
 */
function budgetKinds(qc: QueryClient): readonly BudgetKind[] {
  return qc.getQueryData<DesignView>(keys.design)?.defs.budget_kinds ?? [];
}

export function currentRevision(qc: QueryClient): string | null {
  const design = qc.getQueryData<DesignView>(keys.design);
  const project = qc.getQueryData<ProjectInfo>(keys.project);
  return design?.project.revision ?? project?.revision ?? null;
}

/** 編集の前のバジェットの合計（種類 → 表示文字列） */
type Snapshot = ReadonlyMap<BudgetKind, string>;

function snapshot(qc: QueryClient): Snapshot {
  const s = new Map<BudgetKind, string>();
  for (const k of budgetKinds(qc)) {
    const b = qc.getQueryData<BudgetView>(keys.budget(k));
    if (b?.total) s.set(k, b.total.display);
  }
  return s;
}

/** 変わったバジェットを「質量: 10.84 kg → 10.86 kg」の行にする（表示文字列を並べるだけ） */
async function deltas(qc: QueryClient, before: Snapshot): Promise<string[]> {
  const lines: string[] = [];
  for (const [k, was] of before) {
    const now = await qc.fetchQuery(q.budget(k)).catch(() => null);
    const total = now?.total?.display;
    if (now && total && total !== was)
      lines.push(`${localized(now.label, now.label_en)}: ${was} → ${total}`);
  }
  return lines;
}

/** 編集が通ったとき: 無効化・変更履歴・バジェットの変化と「CLI としてコピー」付きのトースト */
async function onApplied(
  qc: QueryClient,
  r: ApplyResult,
  summary: string,
  paths: string[],
  before: Snapshot,
) {
  clearFieldErrors(paths);
  recordChange({ summary, cli: r.cli, files: r.changed.map((c) => c.path) });
  await Promise.all(invalidatedByOps.map((k) => qc.invalidateQueries({ queryKey: k })));
  const lines = await deltas(qc, before);
  if (lines.length === 0 && r.changed.length > 0)
    lines.push(getT().ops.changed(r.changed.map((c) => c.path).join(", ")));
  const warns = r.diagnostics.filter((d) => d.severity !== "info").length;
  if (warns > 0) lines.push(getT().ops.warnings(warns));
  toast.success(summary, {
    description: lines.join("\n"),
    action:
      r.cli.length > 0
        ? { label: getT().cli.copy, onClick: () => void copyText(r.cli.join("\n")) }
        : undefined,
  });
}

/** 編集が弾かれたとき: 版ずれなら読み直し、検証エラーなら欄の下に理由を出す */
async function onRejected(qc: QueryClient, e: ApiError, summary: string, paths: string[]) {
  if (e.conflict) {
    await Promise.all(invalidatedByOps.map((k) => qc.invalidateQueries({ queryKey: k })));
    toast.error(getT().ops.conflict, { description: getT().ops.conflictDetail });
    return;
  }
  const located = e.diagnostics.flatMap((d) =>
    d.path ? [{ path: d.path, message: d.message }] : [],
  );
  // 診断にパスが無ければ、送った Op のパスにエラー文を付ける（どの欄の編集が弾かれたかを欄の下に出す）
  setFieldErrors(
    located.length > 0 ? located : paths.map((path) => ({ path, message: e.message })),
  );
  toast.error(getT().ops.failed(summary), {
    description: [e.message, ...e.diagnostics.map((d) => `${d.path ?? d.code}: ${d.message}`)]
      .slice(0, 5)
      .join("\n"),
  });
}

/** 編集の mutation の設定（送り先だけが違う。`useEdit` と、hook を通さない検査が使う） */
export function editOptions<R extends { summary: string }>(
  qc: QueryClient,
  send: (req: R, revision: string | null) => Promise<ApplyResult>,
  pathsOf: (req: R) => string[],
): UseMutationOptions<ApplyResult, ApiError, R, Snapshot> {
  return {
    mutationKey: keys.applyOps,
    onMutate: () => snapshot(qc),
    mutationFn: (req) => send(req, currentRevision(qc)),
    onSuccess: (r, req, before) => onApplied(qc, r, req.summary, pathsOf(req), before ?? new Map()),
    onError: (e, req) => onRejected(qc, e, req.summary, pathsOf(req)),
  };
}

/** 編集の mutation の共通形（送り先だけが違う） */
function useEdit<R extends { summary: string }>(
  send: (req: R, revision: string | null) => Promise<ApplyResult>,
  pathsOf: (req: R) => string[],
) {
  const qc = useQueryClient();
  return useMutation<ApplyResult, ApiError, R, Snapshot>(editOptions(qc, send, pathsOf));
}

/** Op を POST /ops で送る mutation の設定（`useApplyOps` と、hook を通さない検査が使う） */
export function applyOpsOptions(qc: QueryClient) {
  return editOptions<OpsRequest>(
    qc,
    ({ ops }, revision) => {
      const body: ApplyRequest = { ops, base_revision: revision, dry_run: false };
      return api<ApplyResult>("/ops", { method: "POST", body });
    },
    (req) => opPaths(req.ops, qc.getQueryData<DesignView>(keys.design)),
  );
}

/** Op を POST /ops で送る唯一の hook */
export function useApplyOps() {
  const qc = useQueryClient();
  return useMutation(applyOpsOptions(qc));
}

/** 人が打った文字列で 1 つの値を書き換える（部品の値とフィールド・設定の表・カタログ品の値とフィールドなど） */
export interface SetTextEdit {
  /** カタログ品の ID。あれば path は品のパラメーターパス（API の `paths` の値）かフィールド名（`label` など） */
  item?: string | null;
  path: string;
  value: string;
  summary: string;
}

/** 人が打ったパスの値を消す（部品の上書き・カタログ品の値・フィールド・設定など） */
export interface UnsetTextEdit {
  /** カタログ品の ID。あれば path は品のパラメーターパス（API の `paths` の値）かフィールド名 */
  item?: string | null;
  path: string;
  summary: string;
}

/** 文字列を service で Op にしてから（適用はしない）`POST /ops` で送る */
async function sendText(
  uri: "/ops/from_text" | "/ops/unset_from_text",
  text: SetText | UnsetText,
  revision: string | null,
): Promise<ApplyResult> {
  const op = await api<Op>(uri, { method: "POST", body: text });
  const body: ApplyRequest = { ops: [op], base_revision: revision, dry_run: false };
  return api<ApplyResult>("/ops", { method: "POST", body });
}

/**
 * 打った文字列を `POST /ops/from_text` で Op にしてから `POST /ops` で送る。値の型（整数・小数・量・真偽・一覧・
 * ベクトル・範囲）と不確かさを決めるのは `fune set`・`fune catalog set` と同じ service の関数（ここでは型を決めない）。
 */
export function useSetText() {
  return useEdit<SetTextEdit>(
    ({ item, path, value }, revision) => {
      const text: SetText = {
        item: item ?? null,
        path,
        value,
        source: null,
        at: null,
        note: null,
        min: null,
        max: null,
        ops: [],
      };
      return sendText("/ops/from_text", text, revision);
    },
    ({ path }) => [path],
  );
}

/** 属性の値を持つもの（部品・カタログ品）。パスは API の `paths`（`RW_X.power.on`・`local/rw_10mNms.power.on`） */
export type AttrOwner =
  | { kind: "part"; owner: Pick<PartRow, "paths"> }
  | { kind: "catalog"; owner: Pick<CatalogItemView, "id" | "paths"> };

/**
 * 属性の値の欄の編集（部品・カタログ品で共通の送り口）。打った文字列とパラメーターパスをそのまま
 * `useSetText`・`useUnsetText` に渡す（値の型は `fune set`・`fune catalog set` と同じ service が決める）。
 */
export function useAttrEdit(target: AttrOwner) {
  const setText = useSetText();
  const unsetText = useUnsetText();
  const item = target.kind === "catalog" ? target.owner.id : null;
  return {
    set: (attr: string, state: string | null, value: string) => {
      const path = paramPath(target.owner, attr, state);
      setText.mutate({ item, path, value, summary: getT().quantity.setSummary(path, value) });
    },
    unset: (attr: string, state: string | null = null) => {
      const path = paramPath(target.owner, attr, state);
      const t = getT();
      const summary = item ? t.catalog.summaryUnset(path) : t.components.attr.unsetSummary(path);
      unsetText.mutate({ item, path, summary });
    },
  };
}

/**
 * 消す先のパスを `POST /ops/unset_from_text` で Op にしてから `POST /ops` で送る。属性の値・フィールド・設定のどれを
 * 消すかを決めるのは `fune unset`・`fune catalog unset` と同じ service の関数。
 */
export function useUnsetText() {
  return useEdit<UnsetTextEdit>(
    ({ item, path }, revision) =>
      sendText("/ops/unset_from_text", { item: item ?? null, path }, revision),
    ({ path }) => [path],
  );
}

/** 名詞のコマンドに打った引数で編集する（ネットの欄・配置の欄など） */
export interface CommandEdit {
  command: CommandText;
  summary: string;
  /** 欄の名前（API が返したパス。弾かれた理由をその欄の下に出す） */
  paths?: string[];
}

/**
 * 欄に打った文字列を `fune net set`・`fune part place` などと同じ引数の形のまま `POST /ops/from_command` で Op にしてから
 * `POST /ops` で送る。量・ベクトル・割合・一覧の読み方は CLI と同じ service の関数（ここでは読まない）。
 */
export function useCommandText() {
  return useEdit<CommandEdit>(
    async ({ command }, revision) => {
      const ops = await api<Op[]>("/ops/from_command", { method: "POST", body: command });
      const body: ApplyRequest = { ops, base_revision: revision, dry_run: false };
      return api<ApplyResult>("/ops", { method: "POST", body });
    },
    ({ paths }) => paths ?? [],
  );
}

/**
 * 打った引数を Op の列にするだけ（書き込まない。`POST /ops/from_command`）。確定前に dry_run を見せるダイアログ・
 * 下書きを当てた計算に使う。command が null なら問い合わせない。打つ間は 250 ms 待ってから問い合わせる。
 * `ops` は今の引数の結果だけ（待っている間と失敗したときは null）。`latest` は待っている間も前の結果を出す（下書きの計算用）。
 */
export function useCommandOps(command: CommandText | null): {
  ops: Op[] | null;
  latest: Op[] | null;
  error: string | null;
} {
  const qc = useQueryClient();
  const revision = currentRevision(qc);
  const now = command ? JSON.stringify(command) : "";
  const json = useDebounced(now, 250);
  const query = useQuery<Op[], ApiError>({
    queryKey: keys.commandOps(revision ?? "", json),
    queryFn: () => api<Op[]>("/ops/from_command", { method: "POST", body: JSON.parse(json) }),
    enabled: json.length > 0,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
    placeholderData: keepPreviousData,
  });
  const settled = json === now && now.length > 0 && !query.isPlaceholderData;
  return {
    ops: settled ? (query.data ?? null) : null,
    latest: now.length > 0 ? (query.data ?? null) : null,
    error: settled ? (query.error?.message ?? null) : null,
  };
}

/** 3D で動かした機体座標の位置・姿勢を送る（取付点に対する相対の Place への換算は service が行う） */
export interface PlaceBodyEdit {
  part: string;
  position: PlaceBodyRequest["position"];
  orientation: PlaceBodyRequest["orientation"];
  configuration?: PlaceBodyRequest["configuration"];
  summary: string;
}

export function usePlaceBody() {
  return useEdit<PlaceBodyEdit>(
    ({ part, position, orientation, configuration }, revision) => {
      const body: Omit<PlaceBodyRequest, "configuration"> &
        Partial<Pick<PlaceBodyRequest, "configuration">> = {
        position,
        orientation,
        configuration: configuration ?? undefined,
        base_revision: revision,
        dry_run: false,
      };
      return api<ApplyResult>(`/parts/${seg(part)}/place_body`, { method: "POST", body });
    },
    () => [],
  );
}

/**
 * Op の確定の最中か。確定が成功すると設計を読み直して revision が進むが、その間も確定したダイアログは開いている。
 * revision が入ったキーの問い合わせ（dry_run）を、この間は新しい revision で取り直さないために見る
 * （取り直すと、消した対象が無くて 404、足した対象が既にあって 409 になる）。
 */
export function useApplying(): boolean {
  return useIsMutating({ mutationKey: keys.applyOps }) > 0;
}

/**
 * 書き込まずに Op を試す（dry_run）。確定前に「CLI としてコピー」と変わるファイルを見せるのに使う。
 * ops が空なら問い合わせない。
 */
export function useDryRun(ops: Op[] | null) {
  const qc = useQueryClient();
  return useQuery(dryRunOptions(qc, ops, useApplying()));
}

/** dry_run の問い合わせの設定（`useDryRun` と、hook を通さない検査が使う）。`applying` の間は問い合わせない */
export function dryRunOptions(qc: QueryClient, ops: Op[] | null, applying: boolean) {
  const revision = currentRevision(qc);
  const json = ops && ops.length > 0 ? JSON.stringify(ops) : "";
  return queryOptions<ApplyResult, ApiError>({
    queryKey: keys.dryRun(revision ?? "", json),
    queryFn: () => {
      const body: ApplyRequest = { ops: ops ?? [], base_revision: revision, dry_run: true };
      return api<ApplyResult>("/ops", { method: "POST", body });
    },
    // 確定の最中は問い合わせない（確定で revision が進んだあとの、同じ Op の取り直しを避ける）
    enabled: json.length > 0 && !applying,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
    // 編集のたびにキーが変わる。次の結果が来るまで前の結果を出しておき、プレビューが空に戻って跳ねないようにする
    placeholderData: keepPreviousData,
  });
}

/**
 * 打った文字列を Op にするだけ（書き込まない。`POST /ops/from_text`）。確定前に dry_run と CLI を見せるダイアログが、
 * service の作った Op を `OpDialog` に渡すのに使う。text が null なら問い合わせない。打つ間は 250 ms 待ってから問い合わせる。
 */
export function useTextOp(text: SetText | null): { op: Op | null; error: string | null } {
  const qc = useQueryClient();
  const revision = currentRevision(qc);
  // 文字列で待つ（毎回作り直すオブジェクトを待つと、描き直すたびに待ち直す）
  const now = text ? JSON.stringify(text) : "";
  const json = useDebounced(now, 250);
  const query = useQuery<Op, ApiError>({
    queryKey: keys.textOp(revision ?? "", json),
    queryFn: () => api<Op>("/ops/from_text", { method: "POST", body: JSON.parse(json) }),
    enabled: json.length > 0,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const settled = json === now && now.length > 0;
  return {
    op: settled ? (query.data ?? null) : null,
    error: settled ? (query.error?.message ?? null) : null,
  };
}

/** 季節解析から新しいケースの Op を作る。保存とプレビューは既存の Op の入口を使う。 */
export function useEclipseCaseOp(id: string, at: string | null, mode: string) {
  const qc = useQueryClient();
  const revision = currentRevision(qc);
  const now = at && id.trim() ? JSON.stringify({ id, at, mode: mode || null }) : "";
  const json = useDebounced(now, 500);
  const query = useQuery<Op, ApiError>({
    queryKey: keys.eclipseCaseOp(revision, json),
    queryFn: () => api<Op>("/ops/case_from_eclipse", { body: JSON.parse(json) }),
    enabled: json.length > 0,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
  return {
    ops: json === now && now && query.data ? [query.data] : null,
    error: json === now ? query.error?.message : null,
  };
}
