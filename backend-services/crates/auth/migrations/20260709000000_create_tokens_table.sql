-- ============================================================================
-- Table: tokens
-- Description:
--   Stores opaque stateful session tokens. Uses soft-deletes (revoked=true)
--   instead of row deletion to preserve a permanent security audit trail.
-- ============================================================================
CREATE TABLE IF NOT EXISTS tokens (
    -- The Base64Url-encoded token string that uniquely identifies the session.
    token TEXT PRIMARY KEY,
    
    -- The UUIDv7 identifier of the user associated with this session.
    user_id UUID NOT NULL,
    
    -- Indicates whether the session has been explicitly revoked or invalidated.
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- The date and time when the session token was generated.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Optimize queries used to track or clean up active sessions for a specific user.
CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id);