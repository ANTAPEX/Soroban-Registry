import * as React from "react";
import { cn } from "@/lib/utils/cn";

export type ButtonVariant = "default" | "outline" | "ghost" | "link";
export type ButtonSize = "sm" | "default" | "lg" | "icon";

const baseStyles =
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-full font-semibold transition-all duration-200 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

const variantStyles: Record<ButtonVariant, string> = {
  default:
    "border-2 border-border-strong bg-primary text-primary-foreground hover:bg-border-strong hover:text-background hover:-translate-y-px",
  outline:
    "border-2 border-border-strong bg-transparent text-foreground hover:bg-border-strong hover:text-background hover:-translate-y-px",
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
  className?: string;
}

/** Composable class-string builder for the Stellar-style pill button — pass
 *  it to any element (Link, a, button) the way shadcn's `buttonVariants` is
 *  used with `asChild`. */
export function buttonVariants({
  variant = "default",
  size = "default",
  className,
}: ButtonVariantsOptions = {}): string {
  return cn(baseStyles, variantStyles[variant], sizeStyles[size], className);
}

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button
      ref={ref}
      className={buttonVariants({ variant, size, className })}
      {...props}
    />
  ),
);
Button.displayName = "Button";
