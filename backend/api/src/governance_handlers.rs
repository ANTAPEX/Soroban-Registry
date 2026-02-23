use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use shared::models::{
    CastVoteRequest, CreateProposalRequest, GovernanceProposal, GovernanceVote, ProposalResults,
    ProposalStatus, VoteDelegation,
};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn create_proposal(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreateProposalRequest>,
) -> ApiResult<Json<GovernanceProposal>> {
    let contract = sqlx::query("SELECT publisher_id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;
    let publisher_id: Uuid = contract.get("publisher_id");

    let now = Utc::now();
    let voting_starts_at = now;
    let voting_ends_at = now + Duration::hours(req.voting_duration_hours as i64);

    let proposal = sqlx::query_as::<_, GovernanceProposal>(
        r#"
        INSERT INTO governance_proposals 
        (contract_id, title, description, governance_model, proposer, voting_starts_at, voting_ends_at, execution_delay_hours)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(&req.governance_model)
    .bind(publisher_id)
    .bind(voting_starts_at)
    .bind(voting_ends_at)
    .bind(req.execution_delay_hours)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create proposal: {}", e)))?;

    Ok(Json(proposal))
}

pub async fn list_proposals(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<GovernanceProposal>>> {
    let proposals = sqlx::query_as::<_, GovernanceProposal>(
        "SELECT * FROM governance_proposals WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(proposals))
}

pub async fn get_proposal(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
) -> ApiResult<Json<GovernanceProposal>> {
    let proposal = sqlx::query_as::<_, GovernanceProposal>(
        "SELECT * FROM governance_proposals WHERE id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("proposal", "Proposal not found"))?;

    Ok(Json(proposal))
}

pub async fn cast_vote(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
    Json(req): Json<CastVoteRequest>,
) -> ApiResult<Json<GovernanceVote>> {
    let proposal = sqlx::query_as::<_, GovernanceProposal>(
        "SELECT * FROM governance_proposals WHERE id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("proposal", "Proposal not found"))?;

    // Get voter (use proposer as placeholder)
    let voter_id = proposal.proposer;

    // Calculate voting power (simplified)
    let voting_power = 1i64;

    let vote = sqlx::query_as::<_, GovernanceVote>(
        r#"
        INSERT INTO governance_votes (proposal_id, voter, vote_choice, voting_power)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (proposal_id, voter) DO UPDATE 
        SET vote_choice = $3, voting_power = $4
        RETURNING *
        "#,
    )
    .bind(proposal_id)
    .bind(voter_id)
    .bind(&req.vote_choice)
    .bind(voting_power)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to cast vote: {}", e)))?;

    Ok(Json(vote))
}

pub async fn get_proposal_results(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
) -> ApiResult<Json<ProposalResults>> {
    let proposal = sqlx::query_as::<_, GovernanceProposal>(
        "SELECT * FROM governance_proposals WHERE id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("proposal", "Proposal not found"))?;

    let votes = sqlx::query(
        r#"
        SELECT 
            SUM(CASE WHEN vote_choice = 'for' THEN voting_power ELSE 0 END) as votes_for,
            SUM(CASE WHEN vote_choice = 'against' THEN voting_power ELSE 0 END) as votes_against,
            SUM(CASE WHEN vote_choice = 'abstain' THEN voting_power ELSE 0 END) as votes_abstain,
            SUM(voting_power) as total_votes
        FROM governance_votes 
        WHERE proposal_id = $1
        "#,
    )
    .bind(proposal_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let votes_for: Option<i64> = votes.get("votes_for");
    let votes_against: Option<i64> = votes.get("votes_against");
    let votes_abstain: Option<i64> = votes.get("votes_abstain");
    let total_votes_opt: Option<i64> = votes.get("total_votes");

    let votes_for = votes_for.unwrap_or(0);
    let votes_against = votes_against.unwrap_or(0);
    let votes_abstain = votes_abstain.unwrap_or(0);
    let total_votes = total_votes_opt.unwrap_or(0);

    let quorum_met = total_votes >= proposal.quorum_required as i64;
    let approval_pct = if total_votes > 0 {
        (votes_for * 100) / total_votes
    } else {
        0
    };
    let approved = quorum_met && approval_pct >= proposal.approval_threshold as i64;

    Ok(Json(ProposalResults {
        proposal,
        votes_for,
        votes_against,
        votes_abstain,
        total_votes,
        quorum_met,
        approved,
    }))
}

pub async fn execute_proposal(
    State(state): State<AppState>,
    Path(proposal_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let results = get_proposal_results(State(state.clone()), Path(proposal_id))
        .await?
        .0;

    if !results.approved {
        return Err(ApiError::bad_request(
            "not_approved",
            "Proposal not approved",
        ));
    }

    sqlx::query(
        "UPDATE governance_proposals SET status = 'executed', executed_at = $1 WHERE id = $2",
    )
    .bind(Utc::now())
    .bind(proposal_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to execute proposal: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delegate_vote(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(delegate_id): Json<Uuid>,
) -> ApiResult<Json<VoteDelegation>> {
    let contract = sqlx::query("SELECT publisher_id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;
    let publisher_id: Uuid = contract.get("publisher_id");

    let delegation = sqlx::query_as::<_, VoteDelegation>(
        r#"
        INSERT INTO vote_delegations (delegator, delegate, contract_id)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(publisher_id)
    .bind(delegate_id)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to delegate vote: {}", e)))?;

    Ok(Json(delegation))
}

pub async fn revoke_delegation(
    State(state): State<AppState>,
    Path(delegation_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    sqlx::query(
        "UPDATE vote_delegations SET active = false, revoked_at = $1 WHERE id = $2",
    )
    .bind(Utc::now())
    .bind(delegation_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to revoke delegation: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}
