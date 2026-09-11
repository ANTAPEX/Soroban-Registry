export type ClassValue = string | number | false | null | undefined;

/** Joins conditional class names, skipping falsy values. */
export function cn(...inputs: ClassValue[]): string {
  return inputs.filter(Boolean).join(" ");
}
