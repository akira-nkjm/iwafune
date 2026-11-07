import { getT } from "@/i18n";
import type { DiagnosticView, ErrorEnvelope } from "./types";

/** API の失敗。`{ error: { code, message }, diagnostics }` を持つ */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly diagnostics: DiagnosticView[];

  constructor(status: number, code: string, message: string, diagnostics: DiagnosticView[] = []) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.diagnostics = diagnostics;
  }

  /** 版ずれ（409）か */
  get conflict(): boolean {
    return this.status === 409;
  }
}

const BASE = "/api/v1";

function isEnvelope(v: unknown): v is ErrorEnvelope {
  if (typeof v !== "object" || v === null || !("error" in v)) return false;
  const e = (v as { error: unknown }).error;
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

/** JSON を送って JSON を受ける。失敗は ApiError にする */
export async function api<T>(path: string, init?: { method?: string; body?: unknown }) {
  let res: Response;
  try {
    res = await fetch(`${BASE}${path}`, {
      method: init?.method ?? (init?.body === undefined ? "GET" : "POST"),
      headers: init?.body === undefined ? undefined : { "content-type": "application/json" },
      body: init?.body === undefined ? undefined : JSON.stringify(init.body),
    });
  } catch (e) {
    throw new ApiError(0, "network", getT().ops.network(String(e)));
  }
  const text = await res.text();
  let json: unknown = null;
  if (text) {
    try {
      json = JSON.parse(text);
    } catch {
      json = null;
    }
  }
  if (!res.ok) {
    if (isEnvelope(json)) {
      const diags = Array.isArray((json as ErrorEnvelope).diagnostics)
        ? (json as ErrorEnvelope).diagnostics
        : [];
      throw new ApiError(res.status, json.error.code, json.error.message, diags);
    }
    throw new ApiError(res.status, "http", `${res.status} ${res.statusText}`);
  }
  return json as T;
}

/** パスの 1 区間を URL に入れる */
export const seg = (s: string) => encodeURIComponent(s);
