import { useEffect, useState } from "react";

/** 待ちの表示（チップ・ロゴ）を出すまでの時間 [ms]。これより短い待ちでは何も足さない */
export const WORKING_DELAY_MS = 1000;

/** 出てから `ms` ミリ秒たったか（0 以下なら最初から true）。速い読み込みで表示がちらつかないために使う */
export function useAfter(ms: number): boolean {
  const [elapsed, setElapsed] = useState(ms <= 0);
  useEffect(() => {
    if (ms <= 0) return;
    const timer = setTimeout(() => setElapsed(true), ms);
    return () => clearTimeout(timer);
  }, [ms]);
  return elapsed;
}

/** 置かれてからの経過秒（整数）。1 秒に 1 回だけ更新する。置かれている間だけ数える */
export function useElapsedSeconds(): number {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const start = Date.now();
    const timer = setInterval(() => setSeconds(Math.floor((Date.now() - start) / 1000)), 1000);
    return () => clearInterval(timer);
  }, []);
  return seconds;
}
