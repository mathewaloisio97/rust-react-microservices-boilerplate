-- ============================================================================
-- Table: user_emails
-- Description:
--   Manages user email addresses, verification states, and the lifecycle of 
--   pending email change requests.
-- ============================================================================
CREATE TABLE IF NOT EXISTS user_emails (
    -- The unique UUIDv7 identifier of the user. One-to-one relationship.
    user_id UUID PRIMARY KEY,
    
    -- The user's active, confirmed email address.
    current_email TEXT NOT NULL,
    
    -- Indicates whether the current_email has been verified by the user.
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Holds the unverified email address during an active email update flow.
    pending_new_email TEXT,
    
    -- The hashed or raw verification token sent to the user.
    verification_code TEXT,
    
    -- Context of the code (e.g., 'VERIFY_CURRENT', 'CONFIRM_NEW').
    verification_type TEXT,
    
    -- Timestamp after which the current verification_code is invalid.
    code_expires_at TIMESTAMPTZ,
    
    -- The date and time when this record was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- The date and time when this record was last modified.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Optimize lookups during login, account recovery, and multi-profile routing.
CREATE INDEX IF NOT EXISTS idx_user_emails_current ON user_emails(current_email);
