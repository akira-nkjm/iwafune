/** 目盛りとツールチップの数（桁を揃えるだけ。単位と倍率は API の Channel.display_unit / display_scale） */
const tick = new Intl.NumberFormat(undefined, { maximumSignificantDigits: 4 });
export const numText = (n: number) => tick.format(n);
