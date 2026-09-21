import {
  addReviewComment,
  createCollaborativeReview,
  fetchCollaborativeReview,
  updateReviewerStatus,
} from "@/lib/api/reviews";
import fetchMock from "jest-fetch-mock";

/**
 * These four calls are keyed by the review id and hang off
 * `/api/reviews/collaborative`, as defined by `collaborative_review_routes()`
 * in the backend's `routes.rs`.
 *
 * Two of them used to point at `/api/contracts/{id}/review` and
 * `/api/contracts/{id}/review/comments`, which no route has ever served, so
 * opening a review session and posting a comment both 404'd. Nothing caught it:
 * the paths type-check, and no test exercised them. These assertions pin each
 * URL to the route that answers it.
 */
describe("collaborative review endpoints", () => {
  const REVIEW_ID = "3f1b0c9e-0000-4000-8000-000000000001";

  beforeEach(() => {
    fetchMock.resetMocks();
    window.localStorage.clear();
    // A mutation first fetches a CSRF token, and `apiFetch` caches it across
    // calls, so which request lands first depends on test order. Answer every
    // request with a body that satisfies both and assert on the paths instead.
    fetchMock.mockResponse(
      JSON.stringify({ token: "test-csrf", review: {}, reviewers: [], comments: [] }),
    );
  });

  function requestedPaths(): string[] {
    return fetchMock.mock.calls.map((call) => String(call[0]));
  }

  it("fetches a review session by review id", async () => {

    await fetchCollaborativeReview(REVIEW_ID);

    expect(requestedPaths()).toContainEqual(
      expect.stringContaining(`/api/reviews/collaborative/${REVIEW_ID}`),
    );
  });

  it("posts a comment to the review, not to the contract", async () => {

    await addReviewComment(REVIEW_ID, { content: "looks good" });

    expect(requestedPaths()).toContainEqual(
      expect.stringContaining(
        `/api/reviews/collaborative/${REVIEW_ID}/comment`,
      ),
    );
    expect(requestedPaths()).not.toContainEqual(
      expect.stringContaining("/review/comments"),
    );
  });

  it("updates reviewer status by review id", async () => {

    await updateReviewerStatus(REVIEW_ID, "approved");

    expect(requestedPaths()).toContainEqual(
      expect.stringContaining(`/api/reviews/collaborative/${REVIEW_ID}/status`),
    );
  });

  it("creates a review session at the collection route", async () => {

    await createCollaborativeReview({
      contract_id: "contract-id",
      version: "1.0.0",
      reviewer_ids: [],
    });

    expect(requestedPaths()).toContainEqual(
      expect.stringContaining("/api/reviews/collaborative"),
    );
  });
});
