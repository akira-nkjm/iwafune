import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/** clsx + tailwind-merge（shadcn/ui と共通の唯一の cn） */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
