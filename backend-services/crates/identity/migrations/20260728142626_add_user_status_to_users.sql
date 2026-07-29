-- ============================================================================
-- Migration: Add status to users
-- Description:
--   Introduces an operational status enum for account lifecycles.
-- ============================================================================

CREATE TYPE user_status AS ENUM (
    'PENDING',
    'ACTIVE',
    'SUSPENDED'
);

ALTER TABLE users 
ADD COLUMN status user_status NOT NULL DEFAULT 'PENDING';
