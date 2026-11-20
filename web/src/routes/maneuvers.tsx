import { type LoaderFunctionArgs, redirect } from "react-router";
import { paths } from "@/lib/nav";

/** 古い URL（マヌーバの独立したページ）は、推進のページのマヌーバのタブへ送る（ID があれば、その行を強調して開く） */
export function clientLoader({ params }: LoaderFunctionArgs) {
  return redirect(params.id ? paths.maneuver(params.id) : paths.maneuvers());
}

export default function Route() {
  return null;
}
