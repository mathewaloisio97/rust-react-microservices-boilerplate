//! API client utilities for gateway communication and authentication flows.

import { GATEWAY_URL } from '../config.js';

export const api = {
  /**
   * Submits a CAPTCHA response token to the gateway for server-side evaluation.
   *
   * @param providerId - The identifier of the verification provider (e.g., 'turnstile', 'recaptcha').
   * @param token - The client-side challenge completion token.
   * @returns A time-limited cryptographic proof-of-humanity voucher string.
   */
  async verifyCaptcha(providerId: string, token: string): Promise<string> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/captcha/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ provider_id: providerId, client_payload: token }),
    });
    if (!res.ok) throw new Error('Bot behavior detected. Verification failed.');
    const data = await res.json();
    return data.captcha_voucher;
  },

  /**
   * Exchanges an opaque session credential for a stateless short-lived JWT access token.
   *
   * @param sessionToken - The active session token acquired via login or OAuth.
   * @returns An object containing the generated access token and its expiration timestamp.
   */
  async mintAccessToken(
    sessionToken: string
  ): Promise<{ access_token: string; expires_at: number }> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/access-tokens`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({ roles: ['user'], ttl_seconds: 900 }),
    });
    if (!res.ok) throw new Error('Failed to mint access token');
    return res.json();
  },

  /**
   * Invalidates the current active session on the gateway server.
   *
   * @param sessionToken - The bearer session token to terminate.
   */
  async logout(sessionToken: string): Promise<void> {
    await fetch(`${GATEWAY_URL}/api/v1/logout`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${sessionToken}` },
    });
  },
};
