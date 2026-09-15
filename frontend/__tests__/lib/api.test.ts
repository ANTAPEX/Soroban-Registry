import { api } from "@/lib/api";
import fetchMock from "jest-fetch-mock";

describe("api", () => {
  beforeEach(() => {
    fetchMock.resetMocks();
    window.localStorage.clear();
  });

  describe("getContracts", () => {
    it("should fetch contracts with default parameters", async () => {
      const mockData = {
        items: [{ id: "1", name: "Test Contract" }],
        total: 1,
        page: 1,
        per_page: 10,
        total_pages: 1,
      };
      fetchMock.mockResponseOnce(JSON.stringify(mockData));

      const result = await api.getContracts();

      expect(result).toEqual(mockData);
      // No params supplied, so no query string: the backend applies its own
      // pagination defaults. The route is /api/contracts — there is no
      // /api/v1/contracts list endpoint.
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/contracts"),
        expect.any(Object),
      );
      expect(fetchMock.mock.calls[0][0]).not.toContain("?");
    });

    it("should send pagination params when supplied", async () => {
      fetchMock.mockResponseOnce(JSON.stringify({ items: [], total: 0 }));

      await api.getContracts({ page: 2, page_size: 10 });

      const url = String(fetchMock.mock.calls[0][0]);
      expect(url).toContain("page=2");
      expect(url).toContain("page_size=10");
    });

    it("should handle API errors gracefully", async () => {
      fetchMock.mockResponseOnce(JSON.stringify({ error: "Not Found" }), {
        status: 404,
      });

      await expect(api.getContracts()).rejects.toThrow();
    });

    it("should handle network failures", async () => {
      fetchMock.mockRejectOnce(new Error("Network failure"));

      await expect(api.getContracts()).rejects.toThrow("Network failure");
    });
  });

  describe("getContract", () => {
    it("should fetch a single contract by id", async () => {
      const mockContract = { id: "test-id", name: "Test Contract" };
      fetchMock.mockResponseOnce(JSON.stringify(mockContract));

      const result = await api.getContract("test-id");

      expect(result).toEqual(mockContract);
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/contracts/test-id"),
        expect.any(Object),
      );
    });
  });

  describe("authenticated mutations", () => {
    it("attaches the canonical stored bearer token", async () => {
      window.localStorage.setItem("soroban_registry_token", "publisher-token");
      // A mutating request first fetches a CSRF token, so the mutation itself
      // is the second call.
      fetchMock.mockResponseOnce(JSON.stringify({ token: "csrf-token" }));
      fetchMock.mockResponseOnce(JSON.stringify({ id: "contract-id" }));

      await api.publishContract({
        contract_id: "CTEST",
        name: "Test",
        network: "testnet",
        publisher_address: "GTEST",
        tags: [],
      });

      expect(String(fetchMock.mock.calls[0][0])).toContain("/api/auth/csrf");

      const headers = new Headers(fetchMock.mock.calls[1][1]?.headers);
      expect(headers.get("Authorization")).toBe("Bearer publisher-token");
      expect(headers.get("Content-Type")).toBe("application/json");
      expect(headers.get("x-csrf-token")).toBe("csrf-token");
    });

    it("does not overwrite an explicit authorization header", async () => {
      window.localStorage.setItem("soroban_registry_token", "stored-token");
      fetchMock.mockResponseOnce(JSON.stringify({ favorites: [] }));

      await api.getPreferences("explicit-token");

      const headers = new Headers(fetchMock.mock.calls[0][1]?.headers);
      expect(headers.get("Authorization")).toBe("Bearer explicit-token");
    });
  });
});
