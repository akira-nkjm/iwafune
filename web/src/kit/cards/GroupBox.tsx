import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { type Tone, toneHex, toneText } from "@/lib/tone";

/**
 * 見出し付きの枠（色の点・小さな前置きの文字・題）。中身はカードの格子（`CardGrid`）と入れ子の枠（`NestedBoxes`）など。
 * `onSelectHeading` を渡すと見出しが押せるボタンになり（空いた所の選択解除を止める）、`selected` で枠に輪を付ける。
 * `nested` は別の枠の中の小さな枠（影なし・地を濃く）。
 */
export function GroupBox({
  tone,
  prefix,
  title,
  nested = false,
  selected = false,
  onSelectHeading,
  children,
}: {
  tone: Tone;
  /** 題の前に置く小さな文字 */
  prefix?: string;
  title: string;
  nested?: boolean;
  selected?: boolean;
  /** 渡すと見出しを押せるボタンにする */
  onSelectHeading?: () => void;
  children?: ReactNode;
}) {
  const heading = (
    <>
      <span
        className="size-2.5 shrink-0 rounded-full"
        style={{ background: toneHex[tone] }}
        aria-hidden
      />
      <span className="text-ink-3">{prefix}</span>
      <span className={cn("font-medium", toneText[tone])}>{title}</span>
    </>
  );
  return (
    <section
      className={cn(
        "rounded-[14px] border border-line p-4",
        nested ? "bg-panel-2/60" : "bg-panel/70 shadow-panel",
        selected && "ring-2 ring-accent",
      )}
    >
      {onSelectHeading ? (
        <button
          type="button"
          className="mb-3 flex items-center gap-2 rounded text-[13px] hover:underline"
          onClick={(e) => {
            e.stopPropagation();
            onSelectHeading();
          }}
        >
          {heading}
        </button>
      ) : (
        <h3 className="mb-3 flex items-center gap-2 text-[13px]">{heading}</h3>
      )}
      {children}
    </section>
  );
}
