import {
  keepPreviousData,
  type QueryClient,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "sonner";
import { getT } from "@/i18n";
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

/** トーストに変化を出すバジェット */
const DELTA_KINDS: readonly BudgetKind[] = ["mass", "power", "data"];

export function currentRevision(qc: QueryClient): string | null {
  const design = qc.getQueryData<DesignView>(keys.design);
  const project = qc.getQueryData<ProjectInfo>(keys.project);
  return design?.project.revision ?? project?.revision ?? null;
}

type Snapshot = Partial<Record<BudgetKind, string>>;

function snapshot(qc: QueryClient): Snapshot {
  const s: Snapshot = {};
  for (const k of DELTA_KINDS) {
    const b = qc.getQueryData<BudgetView>(keys.budget(k));
    if (b) s[k] = b.total.display;
  }
  return s;
}

/** 変わったバジェットを「質量: 10.84 kg → 10.86 kg」の行にする（表示文字列を並べるだけ） */
async function deltas(qc: QueryClient, before: Snapshot): Promise<string[]> {
  const lines: string[] = [];
  for (const k of DELTA_KINDS) {
    const was = before[k];
    if (was === undefined) continue;
    const now = await qc.fetchQuery(q.budget(k)).catch(() => null);
    if (now && now.total.display !== was) lines.push(`${now.label}: ${was} → ${now.total.display}`);
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

/** 編集の mutation の共通形（送り先だけが違う） */
function useEdit<R extends { summary: string }>(
  send: (req: R, revision: string | null) => Promise<ApplyResult>,
  pathsOf: (req: R) => string[],
) {
  const qc = useQueryClient();
  return useMutation<ApplyResult, ApiError, R, Snapshot>({
    onMutate: () => snapshot(qc),
    mutationFn: (req) => send(req, currentRevision(qc)),
    onSuccess: (r, req, before) => onApplied(qc, r, req.summary, pathsOf(req), before ?? {}),
    onError: (e, req) => onRejected(qc, e, req.summary, pathsOf(req)),
  });
}

/** Op を POST /ops で送る唯一の hook */
export function useApplyOps() {
  const qc = useQueryClient();
  return useEdit<OpsRequest>(
    ({ ops }, revision) => {
      const body: ApplyRequest = { ops, base_revision: revision, dry_run: false };
      return api<ApplyResult>("/ops", { method: "POST", body });
    },
    (req) => opPaths(req.ops, qc.getQueryData<DesignView>(keys.design)),
  );
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

/** 3D で動かした機体座標の位置・姿勢を送る（取付点に対する相対の Place への換算は service が行う） */
export interface PlaceBodyEdit {
  part: string;
  position: PlaceBodyRequest["position"];
  orientation: PlaceBodyRequest["orientation"];
  summary: string;
}

export function usePlaceBody() {
  return useEdit<PlaceBodyEdit>(
    ({ part, position, orientation }, revision) => {
      const body: PlaceBodyRequest = {
        position,
        orientation,
        base_revision: revision,
        dry_run: false,
      };
      return api<ApplyResult>(`/parts/${seg(part)}/place_body`, { method: "POST", body });
    },
    () => [],
  );
}

/**
 * 書き込まずに Op を試す（dry_run）。確定前に「CLI としてコピー」と変わるファイルを見せるのに使う。
 * ops が空なら問い合わせない。
 */
export function useDryRun(ops: Op[] | null) {
  const qc = useQueryClient();
  const revision = currentRevision(qc);
  const json = ops && ops.length > 0 ? JSON.stringify(ops) : "";
  return useQuery<ApplyResult, ApiError>({
    queryKey: keys.dryRun(revision ?? "", json),
    queryFn: () => {
      const body: ApplyRequest = { ops: ops ?? [], base_revision: revision, dry_run: true };
      return api<ApplyResult>("/ops", { method: "POST", body });
    },
    enabled: json.length > 0,
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
