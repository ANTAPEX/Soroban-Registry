-- Issue #888: Contract signature verification system.
--
-- Cryptographic authentication of contracts via deployer signatures, with
-- multi-algorithm support (Ed25519, secp256k1/ECDSA), certificate chains,
-- a revocation list, timestamp validity windows, and key rotation.
--
-- Relationship to the existing `package_signatures` (signing_handlers.rs):
-- that table is the Ed25519-only package-signing + transparency-log subsystem.
-- This migration adds the broader verification system (#888) under distinct
-- table names so the two coexist.

-- ── Signing keys (deployer keys + certificate-chain authorities) ──────────────
CREATE TABLE IF NOT EXISTS signing_keys (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Deterministic fingerprint: hex(sha256(algorithm || ':' || raw public key)).
    key_id        TEXT        NOT NULL UNIQUE,
    -- Who controls the key (deployer Stellar address, org id, or CA name).
    owner         TEXT        NOT NULL,
    -- 'ed25519' | 'secp256k1'.
    algorithm     TEXT        NOT NULL,
    -- Base64-encoded public key (32 bytes ed25519; SEC1 compressed/uncompressed secp256k1).
    public_key    TEXT        NOT NULL,
    -- Issuer fingerprint for certificate chains (NULL for self-issued/root).
    parent_key_id TEXT,
    -- Parent's base64 signature over this key's raw public-key bytes (the cert).
    cert_signature TEXT,
    -- Trusted anchor: a root may terminate a chain.
    is_root       BOOLEAN     NOT NULL DEFAULT FALSE,
    -- Validity window for timestamp checks.
    not_before    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    not_after     TIMESTAMPTZ,
    -- 'active' | 'revoked' | 'rotated'.
    status        TEXT        NOT NULL DEFAULT 'active',
    -- Replacement key fingerprint after rotation (old key stays for historical sigs).
    rotated_to    TEXT,
    metadata      JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_signing_keys_owner       ON signing_keys (owner);
CREATE INDEX IF NOT EXISTS idx_signing_keys_parent      ON signing_keys (parent_key_id);
CREATE INDEX IF NOT EXISTS idx_signing_keys_status      ON signing_keys (status);

-- ── Stored contract signatures ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS contract_signatures (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Optional link to a registry contract row.
    contract_id     UUID        REFERENCES contracts(id) ON DELETE CASCADE,
    -- Free-form subject reference (on-chain id, package coordinate, etc.).
    contract_ref    TEXT        NOT NULL,
    -- The exact message/hash that was signed (e.g. the wasm hash).
    subject_hash    TEXT        NOT NULL,
    algorithm       TEXT        NOT NULL,
    -- Base64 signature bytes.
    signature       TEXT        NOT NULL,
    -- Fingerprint of the signing key (joins signing_keys.key_id).
    key_id          TEXT        NOT NULL,
    -- Claimed signing time, and optional validity window.
    signed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    not_before      TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ,
    -- Result of the most recent verification.
    verified        BOOLEAN     NOT NULL DEFAULT FALSE,
    last_verified_at TIMESTAMPTZ,
    metadata        JSONB       NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contract_signatures_contract  ON contract_signatures (contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_key       ON contract_signatures (key_id);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_subject   ON contract_signatures (subject_hash);

-- ── Revocation list ───────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS signature_revocations (
    id            BIGSERIAL   PRIMARY KEY,
    -- Revoked key fingerprint (revokes the key and everything it signed).
    key_id        TEXT,
    -- Or a specific revoked signature.
    signature_id  UUID,
    reason        TEXT        NOT NULL DEFAULT '',
    revoked_by    TEXT,
    revoked_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_signature_revocations_key
    ON signature_revocations (key_id) WHERE key_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_signature_revocations_sig
    ON signature_revocations (signature_id) WHERE signature_id IS NOT NULL;

COMMENT ON TABLE signing_keys IS
    'Deployer/CA keys for the contract signature verification system, incl. cert chains and rotation (issue #888).';
COMMENT ON TABLE contract_signatures IS
    'Stored contract signatures with algorithm, validity window, and verification metadata (issue #888).';
COMMENT ON TABLE signature_revocations IS
    'Revocation list for signing keys and individual signatures (issue #888).';
