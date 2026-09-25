import * as React from "react";
import { cn } from "@/lib/utils/cn";

export type BadgeVariant = "default" | "outline";

const variantStyles: Record<BadgeVariant, string> = {
  default: "bg-accent text-accent-foreground border border-transparent",
  outline: "bg-transparent text-foreground border border-border-strong",
};

export interface BadgeVariantsOptions {
  variant?: BadgeVariant;
  className?: string;
}

export function badgeVariants({
  variant = "default",
  className,
}: BadgeVariantsOptions = {}): string {
  return cn(
    "inline-flex items-center gap-2 rounded-md px-3 py-1.5 font-mono text-xs uppercase tracking-wider",
    variantStyles[variant],
    className,
  );
}

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: BadgeVariant;
}

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={badgeVariants({ variant, className })} {...props} />;
}
