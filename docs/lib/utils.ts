import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** 1434 -> "1,434" */
export function commas(n: number) {
  return n.toLocaleString("en-US");
}

/** Reduction between two token counts, as a whole percentage. */
export function reduction(before: number, after: number) {
  return Math.round((1 - after / before) * 100);
}
