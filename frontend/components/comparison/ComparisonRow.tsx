"use client";

import type { CellTone } from "@/utils/comparison";

type Props = {
  label: string;
  values: Array<{ contractId: string; display: string; tone: CellTone }>;
};

function toneClass(tone: CellTone) {
  if (tone === "best")
    return "bg-success/10 text-success border-success/30";
  if (tone === "worst")
    return "bg-danger/10 text-danger border-danger/30";
  if (tone === "different")
    return "bg-primary/10 text-primary border-primary/30";
  return "bg-transparent text-foreground border-border";
}

export default function ComparisonRow({ label, values }: Props) {
  return (
    <tr className="border-t border-border">
      <th
        scope="row"
        className="text-left text-xs font-semibold text-muted-foreground px-3 py-2 bg-accent/40"
      >
        {label}
      </th>
      {values.map((v) => (
        <td key={v.contractId} className="px-3 py-2">
          <div
            className={`rounded-lg border px-2 py-1.5 text-xs ${toneClass(v.tone)}`}
          >
            {v.display}
          </div>
        </td>
      ))}
    </tr>
  );
}
