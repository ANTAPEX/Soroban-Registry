// tests/usage_counter_integration_tests.rs
//
// Integration tests for the usage counter.
//
// The behaviour worth testing here — that an increment survives a round trip
// through Postgres, and that concurrent increments do not lose updates — cannot
// be checked without a database. These tests are therefore #[ignore]d by
// default, matching the convention used by the other DB-backed suites in this
// directory (see dependency_graph_tests.rs for the full setup recipe).
//
// To run:
//   docker run -d --name sr-test-pg -e POSTGRES_PASSWORD=postgres \
//     -e POSTGRES_DB=soroban_registry -p 5432:5432 postgres:16
//   export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"
//   cargo test --test usage_counter_integration_tests -- --include-ignored
//
// They body out to todo!() rather than to a vacuous assert!(true). The previous
// version passed when run with --include-ignored despite touching no database,
// which reported coverage that did not exist. Failing loudly is the honest
// signal that the work is outstanding.
//
// Argument types for every usage_counter entry point are pinned by a unit test
// in api/src/usage_counter.rs, which needs no database.

#[cfg(test)]
mod tests {
    /// Increment once against a real row and read the value back.
    ///
    /// Spec: insert a contract with usage_count 0, call
    /// `increment_usage_counter`, then assert the persisted count is 1.
    #[tokio::test]
    #[ignore = "requires a running Postgres"]
    async fn increment_persists_to_the_database() {
        todo!("seed a contract, increment, assert usage_count == 1");
    }

    /// Concurrent increments must not lose updates.
    ///
    /// Spec: seed one contract, spawn 10 concurrent `increment_usage_counter`
    /// calls, await all of them, then assert the persisted count is exactly 10.
    /// This is the test that would catch a read-modify-write regression in the
    /// SQL, which is why the production statement is a single atomic UPDATE.
    #[tokio::test]
    #[ignore = "requires a running Postgres"]
    async fn concurrent_increments_do_not_lose_updates() {
        todo!("seed a contract, run 10 concurrent increments, assert count == 10");
    }

    /// `ContractStatsResponse` survives a JSON round trip.
    ///
    /// Needs no database, so it is not ignored.
    #[test]
    fn contract_stats_response_round_trips_through_json() {
        use api::handlers::ContractStatsResponse;
        use chrono::Utc;
        use uuid::Uuid;

        let response = ContractStatsResponse {
            contract_id: Uuid::new_v4(),
            usage_count: 42,
            last_accessed_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        let deserialized: ContractStatsResponse =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.contract_id, response.contract_id);
        assert_eq!(deserialized.usage_count, response.usage_count);
    }
}
