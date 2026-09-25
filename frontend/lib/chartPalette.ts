/**
 * Chart and graph colours, as CSS variable references.
 *
 * The values are defined per theme in app/globals.css (--chart-*), so a
 * chart that uses these follows light and dark mode automatically. They
 * work anywhere a colour lands on an SVG fill/stroke (Recharts, d3).
 * Canvas APIs cannot resolve var(); use resolveCssColor for those.
 */
export const CHART_SERIES = [
  "var(--chart-1)",
  "var(--chart-2)",
  "var(--chart-3)",
  "var(--chart-4)",
  "var(--chart-5)",
  "var(--chart-6)",
] as const;

/**
 * Keeps a category breakdown within the series palette. With more
 * categories than colours the palette would repeat, and two slices would
 * share a legend colour, so the largest (CHART_SERIES.length - 1) are kept
 * and the rest fold into one "Other" entry, which takes the last colour.
 */
export function foldIntoOther<T extends { category: string; count: number }>(
  data: T[],
): Array<{ category: string; count: number }> {
  if (data.length <= CHART_SERIES.length) return data;
  const sorted = [...data].sort((a, b) => b.count - a.count);
  const keep = sorted.slice(0, CHART_SERIES.length - 1);
  const rest = sorted.slice(CHART_SERIES.length - 1);
  return [
    ...keep,
    { category: "Other", count: rest.reduce((sum, item) => sum + item.count, 0) },
  ];
}

export const CHART_GRID = "var(--chart-grid)";
export const CHART_AXIS = "var(--chart-axis)";

export const GRAPH_COLORS = {
  accent: "var(--chart-1)",
  success: "var(--success)",
  danger: "var(--danger)",
  info: "var(--chart-3)",
  highlight: "var(--primary)",
  edge: "var(--border-strong)",
  label: "var(--muted-foreground)",
  labelStrong: "var(--foreground)",
  nodeStroke: "var(--background)",
  muted: "var(--chart-6)",
} as const;

export const NETWORK_COLORS: Record<string, string> = {
  mainnet: "var(--chart-2)",
  testnet: "var(--chart-3)",
  futurenet: "var(--chart-4)",
};

/** Resolves a var(--x) reference to a concrete colour, for canvas use. */
export function resolveCssColor(value: string, fallback = "#0e131c"): string {
  if (typeof window === "undefined") return fallback;
  const match = /^var\((--[\w-]+)\)$/.exec(value);
  if (!match) return value;
  const resolved = getComputedStyle(document.documentElement)
    .getPropertyValue(match[1])
    .trim();
  return resolved || fallback;
}

const THEME_VARS = [
  "--chart-1",
  "--chart-2",
  "--chart-3",
  "--chart-4",
  "--chart-5",
  "--chart-6",
  "--chart-grid",
  "--chart-axis",
  "--success",
  "--danger",
  "--primary",
  "--border-strong",
  "--muted-foreground",
  "--foreground",
  "--background",
  "--card",
  "--border",
];

/**
 * Serializes an SVG for export with the current theme's colour variables
 * copied onto its root. A standalone SVG file or image has no access to
 * the page's CSS, so var(--chart-1) would otherwise resolve to nothing.
 */
export function serializeSvgWithTheme(svg: SVGSVGElement): string {
  const clone = svg.cloneNode(true) as SVGSVGElement;
  const computed = getComputedStyle(document.documentElement);
  THEME_VARS.forEach((name) => {
    const value = computed.getPropertyValue(name).trim();
    if (value) clone.style.setProperty(name, value);
  });
  return new XMLSerializer().serializeToString(clone);
}
