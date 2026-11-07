import { useEffect, useState } from "react";

/** 値の変化を ms だけ遅らせる（入力中に毎打鍵で問い合わせないため） */
export function useDebounced<T>(value: T, ms = 200): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}
