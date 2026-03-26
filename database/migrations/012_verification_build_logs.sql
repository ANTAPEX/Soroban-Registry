-- Add build_logs column to verifications table for storing compilation output
ALTER TABLE verifications ADD COLUMN IF NOT EXISTS build_logs TEXT;

-- Index for faster lookup of verifications by status
CREATE INDEX IF NOT EXISTS idx_verifications_status_created ON verifications(status, created_at DESC);
