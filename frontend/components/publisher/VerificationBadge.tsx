import React from "react";
import { CheckCircle, XCircle, Clock } from "lucide-react";

interface VerificationBadgeProps {
  status: "verified" | "failed" | "pending";
}

export function VerificationBadge({ status }: VerificationBadgeProps) {
  const config = {
    verified: {
      icon: CheckCircle,
      text: "Verified",
      className:
        "bg-success/10 text-success border-success/30",
    },
    failed: {
      icon: XCircle,
      text: "Failed",
      className:
        "bg-danger/10 text-danger border-danger/30",
    },
    pending: {
      icon: Clock,
      text: "Pending",
      className:
        "bg-primary/10 text-primary border-primary/30",
    },
  };

  const { icon: Icon, text, className } = config[status] || config.pending;

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium border ${className}`}
    >
      <Icon className="w-3.5 h-3.5" />
      {text}
    </span>
  );
}
