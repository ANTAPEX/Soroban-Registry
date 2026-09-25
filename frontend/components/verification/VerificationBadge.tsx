"use client";

import React from 'react';
import { CheckCircle2, ShieldAlert, ShieldCheck, ShieldX } from 'lucide-react';
import type { VerificationStatus } from '@/types/verification';
import { useTranslation } from '@/lib/i18n/client';
import type { TFunction } from 'i18next';

function getBadgeConfig(
  status: VerificationStatus,
  t: TFunction,
  level?: string,
  driftStatus?: string,
): {
  label: string;
  className: string;
  Icon: React.ComponentType<{ className?: string }>;
} {
  // CRITICAL REQUIREMENT (Issue #1191):
  // Drift must suppress or visibly qualify the verified badge rather than sitting quietly beside it.
  // The contract detail page cannot show an unqualified "verified" badge while drift is recorded.
  if (status === "approved" && driftStatus === "drift") {
    return {
      label: "Verified (Drift Detected)",
      className: "bg-red-500/15 text-red-600 dark:text-red-400 border-red-500/40 animate-pulse font-bold",
      Icon: ShieldAlert,
    };
  }

  if (status === "approved" && driftStatus === "unknown") {
    return {
      label: "Verified (Drift Unknown)",
      className: "bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/30",
      Icon: ShieldAlert,
    };
  }

  if (status === "approved" && driftStatus === "not_on_chain") {
    return {
      label: "Verified (Not On Chain)",
      className: "bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/30",
      Icon: ShieldX,
    };
  }

  switch (status) {
    case "approved": {
      const levelLabel = level
        ? ` (${level.charAt(0).toUpperCase()}${level.slice(1)})`
        : "";
      return {
        label: `${t("verificationBadge.verified")}${levelLabel}`,
        className: "bg-green-500/10 text-green-500 border-green-500/20",
        Icon: CheckCircle2,
      };
    }
    case "under_review":
      return {
        label: t("verificationBadge.underReview"),
        className: "bg-blue-500/10 text-blue-500 border-blue-500/20",
        Icon: ShieldCheck,
      };
    case "rejected":
      return {
        label: t("verificationBadge.rejected"),
        className: "bg-red-500/10 text-red-500 border-red-500/20",
        Icon: ShieldX,
      };
    case "submitted":
      return {
        label: t("verificationBadge.submitted"),
        className: "bg-yellow-500/10 text-yellow-600 border-yellow-500/20",
        Icon: ShieldAlert,
      };
    default:
      return {
        label: t("verificationBadge.draft"),
        className: "bg-muted text-muted-foreground border-border",
        Icon: ShieldAlert,
      };
  }
}

interface VerificationBadgeProps {
  status: VerificationStatus;
  level?: string;
  size?: "sm" | "md";
  driftStatus?: string;
}

export default function VerificationBadge({
  status,
  level,
  size = "sm",
  driftStatus,
}: VerificationBadgeProps) {
  const { t } = useTranslation("common");
  const cfg = getBadgeConfig(status, t, level, driftStatus);

  const iconSize = size === "md" ? "w-4 h-4" : "w-3 h-3";
  const textSize = size === "md" ? "text-xs" : "text-[10px]";
  const padding = size === "md" ? "px-2.5 py-1" : "px-2 py-0.5";

  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full border ${padding} ${textSize} font-semibold uppercase tracking-wide ${cfg.className}`}
    >
      <cfg.Icon className={iconSize} />
      {cfg.label}
    </span>
  );
}
