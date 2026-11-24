/**
 * 配置のアルゴリズムの版（features/diagram/layout/plan-key.ts の LAYOUT_VERSION）ごとの、結果の写し
 * （layout-cases.ts の入力を配置した結果の SHA-256。layout-identity.test.ts が照合する）。
 * 配置の結果が変わる変更をしたら、LAYOUT_VERSION を上げ、新しい版の行を足す（前の版の行は消さない）。
 */
export const LAYOUT_RESULT_DIGESTS: Record<number, string> = {
  1: "9850d640c59443e8ba185318a80f8ef89799f932bb02fa146bf628948ddf6e02",
};
