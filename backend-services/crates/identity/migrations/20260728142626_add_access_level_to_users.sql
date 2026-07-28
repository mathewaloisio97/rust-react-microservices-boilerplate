-- ============================================================================
-- Migration: Add access_level and status to users
-- Description:
--   Introduces RBAC tiers and an operational status enum for account lifecycles.
-- ============================================================================

CREATE TYPE access_level AS ENUM (
    'DEFAULT',
    'STAFF',
    'ADMIN',
    'SUPER_ADMIN',
    'SYSTEM'
);
CREATE TYPE user_status AS ENUM (
    'PENDING',
    'ACTIVE',
    'SUSPENDED'
);

ALTER TABLE users 
ADD COLUMN access_level access_level NOT NULL DEFAULT 'DEFAULT',
ADD COLUMN status user_status NOT NULL DEFAULT 'PENDING';
