import { fetchAllContracts } from "@/lib/api/contracts";
import fetchMock from "jest-fetch-mock";

/**
 * `fetchAllContracts` replaces a paging loop that `ContractImportExportPanel`
 * used to run inline. What is worth pinning is the part the component depended
 * on: every page is requested, in order, and the progress callback reports a
 * total that only becomes known after the first response.
 */
describe("fetchAllContracts", () => {
  beforeEach(() => {
    fetchMock.resetMocks();
  });

  function page(pageNumber: number, totalPages: number) {
    return JSON.stringify({
      items: [{ id: `contract-${pageNumber}` }],
      total: totalPages,
      page: pageNumber,
      page_size: 100,
      total_pages: totalPages,
    });
  }

  function requestedPages(): number[] {
    return fetchMock.mock.calls.map((call) => {
      const url = new URL(String(call[0]), "https://example.test");
      return Number(url.searchParams.get("page"));
    });
  }

  it("requests every page in order and concatenates the items", async () => {
    fetchMock.mockResponses(page(1, 3), page(2, 3), page(3, 3));

    const items = await fetchAllContracts({ page_size: 100 });

    expect(requestedPages()).toEqual([1, 2, 3]);
    expect(items.map((c) => c.id)).toEqual([
      "contract-1",
      "contract-2",
      "contract-3",
    ]);
  });

  it("stops after one request when there is a single page", async () => {
    fetchMock.mockResponses(page(1, 1));

    const items = await fetchAllContracts();

    expect(fetchMock.mock.calls).toHaveLength(1);
    expect(items).toHaveLength(1);
  });

  it("reports progress from page one, with the total the first page gave", async () => {
    fetchMock.mockResponses(page(1, 2), page(2, 2));
    const seen: Array<[number, number]> = [];

    await fetchAllContracts({}, (current, total) => seen.push([current, total]));

    expect(seen).toEqual([
      [1, 2],
      [2, 2],
    ]);
  });

  it("treats a total_pages of zero as one page", async () => {
    fetchMock.mockResponses(page(1, 0));
    const seen: Array<[number, number]> = [];

    await fetchAllContracts({}, (current, total) => seen.push([current, total]));

    expect(fetchMock.mock.calls).toHaveLength(1);
    expect(seen).toEqual([[1, 1]]);
  });
});
