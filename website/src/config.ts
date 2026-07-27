//! Global frontend configuration and environment variables.

/**
 * Edge Gateway API base URL.
 * Defaults to the local docker/development gateway if not explicitly configured.
 */
export const GATEWAY_URL = import.meta.env.VITE_GATEWAY_URL || 'http://localhost:3000';

const rawTurnstileKey = import.meta.env.VITE_TURNSTILE_SITE_KEY?.trim();
/**
 * Cloudflare Turnstile public site key.
 * Forcefully uses the official "Always Passes" dummy site key in local dev mode
 * to guarantee compatibility with backend local-dev verification secrets.
 */
export const TURNSTILE_SITE_KEY = import.meta.env.DEV
  ? '1x00000000000000000000AA'
  : rawTurnstileKey || '';

/**
 * Google OAuth Public Client ID.
 * Required for the Google Identity Services popup to initialize successfully.
 */
export const GOOGLE_CLIENT_ID = import.meta.env.VITE_GOOGLE_CLIENT_ID || '';
