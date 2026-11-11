import type { FocusEvent, KeyboardEvent } from "react";
import { cn } from "@/lib/cn";
import { TextInput, type TextInputProps } from "./TextInput";

/** CommitInput の Props。打っている間の値は欄が持ち、確定したときだけ呼び手に渡す */
export type CommitInputProps = Omit<
  TextInputProps,
  "value" | "defaultValue" | "onChange" | "onBlur" | "onKeyDown" | "aria-label"
> & {
  /** いまの値（保存されている文字列）。変わると欄の文字も戻す */
  value: string;
  /** 確定: 前後の空白を除いた文字列。`value` と同じなら呼ばない */
  onCommit: (text: string) => void;
  /** 欄の名前（aria-label） */
  label: string;
  /** false なら、空にして離れたときに確定せず `value` へ戻す（空で消せない欄。員数など）。既定は true（空で確定 = 消す） */
  allowEmpty?: boolean;
};

/**
 * 欄を離れたとき（Enter でも）に確定する 1 行の入力欄。Escape は打った文字を捨てて `value` に戻す。
 * 表や詳細の行の中の欄（既定は `size="sm"`）。読み取り専用（`disabled`）のときは枠を消して文字だけに見せる
 */
export function CommitInput({
  value,
  onCommit,
  label,
  allowEmpty = true,
  size = "sm",
  className,
  ...rest
}: CommitInputProps) {
  const commit = (event: FocusEvent<HTMLInputElement>) => {
    const text = event.currentTarget.value.trim();
    if (!allowEmpty && !text) event.currentTarget.value = value;
    else if (text !== value) onCommit(text);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") event.currentTarget.value = value;
    else if (event.key !== "Enter") return;
    event.preventDefault();
    event.currentTarget.blur();
  };
  return (
    <TextInput
      {...rest}
      // 保存された値が変わったら欄を作り直して、打っていない新しい値を出す
      key={value}
      size={size}
      aria-label={label}
      defaultValue={value}
      onBlur={commit}
      onKeyDown={onKeyDown}
      className={cn("disabled:border-transparent disabled:bg-transparent", className)}
    />
  );
}
