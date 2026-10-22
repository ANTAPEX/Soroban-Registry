"use client";

import React from "react";
import { AlertTriangle, CheckCircle2, HelpCircle, ShieldX } from "lucide-react";

export type DriftStatus = "match" | "drift" | "not_on_chain" | "unknown";

interface DriftBadgeProps {
  status: DriftStatus | string;
  size?: "sm" | "md";
  className?: string;
}

export default function DriftBadge({
  status,
  size = "sm",
  className = "",
}: DriftBadgeProps) {
  const iconSize = size === "md" ? "w-4 h-4" : "w-3 h-3";
  const textSize = size === "md" ? "text-xs" : "text-[10px]";
  const padding = size === "md" ? "px-2.5 py-1" : "px-2 py-0.5";

  switch (status) {
    case "match":
      return (
        <span
          className={`inline-flex items-center gap-1 rounded-full border border-green-500/30 bg-green-500/10 text-green-600 dark:text-green-400 font-semibold uppercase tracking-wide ${padding} ${textSize} ${className}`}
        >
          <CheckCircle2 className={iconSize} />
          WASM Match
        </span>
      );
    case "drift":
      return (
        <span
          className={`inline-flex items-center gap-1 rounded-full border border-red-500/30 bg-red-500/15 text-red-600 dark:text-red-400 font-bold uppercase tracking-wide shadow-sm animate-pulse ${padding} ${textSize} ${className}`}
        >
          <AlertTriangle className={iconSize} />
          Drift Detected
        </span>
      );
    case "not_on_chain":
      return (
        <span
          className={`inline-flex items-center gap-1 rounded-full border border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400 font-semibold uppercase tracking-wide ${padding} ${textSize} ${className}`}
        >
          <ShieldX className={iconSize} />
          Not On Chain
        </span>
      );
    case "unknown":
    default:
      return (
        <span
          className={`inline-flex items-center gap-1 rounded-full border border-slate-500/30 bg-slate-500/10 text-slate-600 dark:text-slate-400 font-semibold uppercase tracking-wide ${padding} ${textSize} ${className}`}
        >
          <HelpCircle className={iconSize} />
          Drift Unknown
        </span>
      );
  }
}
