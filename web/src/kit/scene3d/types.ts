/** 3D の場面で共通の小さな型（機体座標系 [m] の点・向き、視点を寄せる依頼）。 */

export type V3 = [number, number, number];

/** 3D の視点を部品に寄せる依頼。同じ部品を続けて押しても寄せ直すよう、押した回数 `n` を持つ */
export interface FocusRequest {
  id: string;
  n: number;
}
