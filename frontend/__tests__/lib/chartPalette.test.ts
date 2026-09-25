import { CHART_SERIES, foldIntoOther } from "@/lib/chartPalette";

const categories = (counts: number[]) =>
  counts.map((count, i) => ({ category: `c${i}`, count }));

test("foldIntoOther leaves a breakdown that fits the palette unchanged", () => {
  const data = categories([3, 2, 1]);
  expect(foldIntoOther(data)).toBe(data);
});

test("foldIntoOther keeps the largest categories and sums the rest", () => {
  const data = categories([1, 9, 2, 8, 3, 7, 4, 6]);
  const folded = foldIntoOther(data);

  expect(folded).toHaveLength(CHART_SERIES.length);
  expect(folded.slice(0, -1).map((d) => d.count)).toEqual([9, 8, 7, 6, 4]);
  expect(folded[folded.length - 1]).toEqual({ category: "Other", count: 1 + 2 + 3 });
});
