import * as React from "react";
import { cn } from "@/lib/utils/cn";

export type ButtonVariant = "default" | "outline" | "ghost" | "link";
export type ButtonSize = "sm" | "default" | "lg" | "icon";

const baseStyles =
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md font-medium transition-all duration-200 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

const variantStyles: Record<ButtonVariant, string> = {
  default:
    "border border-primary bg-primary text-primary-foreground hover:border-primary-gradient-end hover:bg-primary-gradient-end",
  outline:
    "border border-border-strong bg-transparent text-foreground hover:border-foreground hover:bg-accent",
  ghost: "text-foreground hover:bg-accent",
  link: "text-primary underline-offset-4 hover:underline",
};

const sizeStyles: Record<ButtonSize, string> = {
  sm: "h-8 px-3.5 text-[13px]",
  default: "h-10 px-5 text-sm",
  lg: "h-12 px-7 text-base",
  icon: "h-9 w-9",
};

export interface ButtonVariantsOptions {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Nudge the button up 1px on hover. Set false for buttons whose own
   *  position already relies on a `translate-y` utility (e.g. absolutely
   *  centered inside a search field) — otherwise the hover state's
   *  transform overwrites the centering transform instead of composing
   *  with it, since both target the same `--tw-translate-y` variable. */
  lift?: boolean;
  className?: string;
}

/** Composable class-string builder for the squared ledger-style button — pass
 *  it to any element (Link, a, button) the way shadcn's `buttonVariants` is
 *  used with `asChild`. */
export function buttonVariants({
  variant = "default",
  size = "default",
  lift = true,
  className,
}: ButtonVariantsOptions = {}): string {
  return cn(
    baseStyles,
    variantStyles[variant],
    sizeStyles[size],
    lift && "motion-safe:hover:-translate-y-px",
    className,
  );
}

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  lift?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, lift, ...props }, ref) => (
    <button
      ref={ref}
      className={buttonVariants({ variant, size, lift, className })}
      {...props}
    />
  ),
);
Button.displayName = "Button";
