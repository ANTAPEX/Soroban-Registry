-- Trust scoring
CREATE TABLE trust_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    score INTEGER NOT NULL CHECK (score >= 0 AND score <= 100),
    verified_points INTEGER NOT NULL DEFAULT 0,
    audit_points INTEGER NOT NULL DEFAULT 0,
    usage_points INTEGER NOT NULL DEFAULT 0,
    age_points INTEGER NOT NULL DEFAULT 0,
    vulnerability_penalty INTEGER NOT NULL DEFAULT 0,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id)
);

CREATE INDEX idx_trust_scores_contract_id ON trust_scores(contract_id);
CREATE INDEX idx_trust_scores_score ON trust_scores(score);
CREATE INDEX idx_trust_scores_calculated_at ON trust_scores(calculated_at);
