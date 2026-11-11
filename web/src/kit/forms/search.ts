/**
 * 検索欄の文字列に一致するか（前後の空白を除き、大文字と小文字を区別しない部分一致）。`values` のどれかに含まれれば一致。
 * 空の検索は何にでも一致する。null の値は見ない（未設定の欄）
 */
export function matchesSearch(search: string, values: readonly (string | null)[]): boolean {
  const s = search.trim().toLowerCase();
  if (!s) return true;
  return values.some((value) => value?.toLowerCase().includes(s) ?? false);
}
