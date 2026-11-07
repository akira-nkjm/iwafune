import { useFormatted } from "./use-formatted";

/** SI の値を、サーバーが選んだ単位と桁で出す。届くまでは fallback。suffix は dB のように数に添える文字 */
export function Formatted({
  si,
  dimension,
  unit = null,
  fallback,
  suffix = null,
}: {
  si: number;
  dimension: string;
  unit?: string | null;
  fallback: string;
  suffix?: string | null;
}) {
  const text = useFormatted(si, dimension, unit);
  return (
    <span className="num">
      {text ?? fallback}
      {suffix ? ` ${suffix}` : ""}
    </span>
  );
}
