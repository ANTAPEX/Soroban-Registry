import React from "react";
import { PublisherResponse } from "@/types/publisher";
import { Award, ShieldCheck, ShieldAlert, BarChart2 } from "lucide-react";

interface PublisherStatsProps {
  publisher: PublisherResponse;
}

export function PublisherStats({ publisher }: PublisherStatsProps) {
  const successRate =
    publisher.totalContracts > 0
      ? (publisher.verifiedContracts / publisher.totalContracts) * 100
      : 0;

  const getSuccessColor = (rate: number) => {
    if (rate >= 70) return "text-success bg-success/10";
    if (rate >= 40) return "text-primary bg-primary/10";
    return "text-danger bg-danger/10";
  };

  const statItems = [
    {
      label: "Total contracts",
      value: publisher.totalContracts,
      icon: BarChart2,
      color: "text-primary bg-primary/10",
    },
    {
      label: "Verification success",
      value: `${successRate.toFixed(1)}%`,
      icon: Award,
      color: getSuccessColor(successRate),
    },
    {
      label: "Verified contracts",
      value: publisher.verifiedContracts,
      icon: ShieldCheck,
      color: "text-success bg-success/10",
    },
    {
      label: "Failed verifications",
      value: publisher.failedVerifications,
      icon: ShieldAlert,
      color: "text-danger bg-danger/10",
    },
  ];

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      {statItems.map((item) => (
        <div
          key={item.label}
          className="bg-card p-4 rounded-lg border border-border"
        >
          <div className={`p-3 rounded-lg w-fit mb-3 ${item.color}`}>
            <item.icon className="w-6 h-6" />
          </div>
          <p className="text-sm text-muted-foreground font-medium">
            {item.label}
          </p>
          <p className="text-2xl font-semibold font-mono tabular-nums text-foreground mt-1">
            {item.value}
          </p>
        </div>
      ))}
    </div>
  );
}
