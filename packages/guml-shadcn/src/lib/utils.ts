import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * Merge class names, with later Tailwind utilities winning over earlier conflicting ones.
 *
 * shadcn's own helper, and every component here imports it. `clsx` handles conditionals and arrays;
 * `tailwind-merge` resolves conflicts, so `cn("px-2", "px-4")` is `px-4` rather than both — which is what
 * makes a component's `className` prop able to *override* its defaults instead of fighting them.
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
