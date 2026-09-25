import React from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { CHART_SERIES } from "@/lib/chartPalette";

interface TopContractPoint {
  id: string;
  name: string;
  interaction_count: number;
}

const COLORS = CHART_SERIES;

export default function TopContractsBarChart({
  data,
}: {
  data: TopContractPoint[];
}) {
  if (!data || data.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
        No top contract data available
      </div>
    );
  }

  const normalized = [...data]
    .sort((a, b) => b.interaction_count - a.interaction_count)
    .slice(0, 10)
    .map((item) => ({
      ...item,
      shortName:
        item.name.length > 16 ? `${item.name.slice(0, 16)}...` : item.name,
    }));

  return (
    <div className="h-[300px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          data={normalized}
          margin={{ top: 8, right: 12, left: 0, bottom: 8 }}
        >
          <CartesianGrid
            strokeDasharray="3 3"
            vertical={false}
            stroke="var(--border)"
          />
          <XAxis
            dataKey="shortName"
            interval={0}
            angle={-25}
            textAnchor="end"
            height={56}
            tick={{ fill: "var(--muted-foreground)", fontSize: 11 }}
            tickLine={false}
            axisLine={false}
          />
          <YAxis
            tick={{ fill: "var(--muted-foreground)", fontSize: 12 }}
            tickLine={false}
            axisLine={false}
          />
          <Tooltip
            cursor={{ fill: "var(--muted)", opacity: 0.35 }}
            contentStyle={{
              backgroundColor: "var(--card)",
              borderColor: "var(--border)",
              borderRadius: "0.5rem",
            }}
            formatter={(value, _name, payload) => {
              const safeValue =
                typeof value === "number" ? value : Number(value || 0);
              const label = payload?.payload?.name || "Contract";
              return [safeValue.toLocaleString(), label];
            }}
          />
          <Bar dataKey="interaction_count" radius={[8, 8, 0, 0]}>
            {normalized.map((entry, index) => (
              <Cell key={entry.id} fill={COLORS[index % COLORS.length]} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
