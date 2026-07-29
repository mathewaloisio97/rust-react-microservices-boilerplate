/**
 * API client utilities for gateway communication and authentication flows.
 *
 * Provides typed abstraction functions for interacting with public HTTP API Gateway
 * endpoints, handling captcha verification, authentication lifecycle actions, access token
 * minting, and email configuration management.
 *
 * @module api
 */

import { GATEWAY_URL } from '../config.js';

/**
 * Return structure when minting a short-lived access JWT token.
 */
export interface AccessTokenResponse {
  /** Signed JWT access token containing user claims and assigned scopes. */
  access_token: string;
  /** Unix epoch timestamp in seconds indicating token expiration. */
  expires_at: number;
}

/**
 * Account email configuration and state machine response.
 */
export interface EmailStateResponse {
  /** The unique user identifier (UUID v7). */
  user_id: string;
  /** Current confirmed primary email address. */
  current_email: string;
  /** Whether the primary email is verified. */
  is_verified: boolean;
  /** Staged email address awaiting confirmation; empty if none pending. */
  pending_new_email: string;
  /** Current stage in the email verification state machine. */
  verification_type: string;
  /** Indicates if the user is authorized to change their email. */
  can_change_email: boolean;
  /** Primary credential provider type (e.g., 'local', 'google', 'apple'). */
  provider: string;
}

/**
 * Response payload following an email change request.
 */
export interface SetEmailResponse {
  /** State machine status string (e.g., 'UNVERIFIED', 'ALREADY_VERIFIED'). */
  status: string;
}

/**
 * Response payload following a public challenge code verification attempt.
 */
export interface VerifyEmailResponse {
  /** True if the code challenge was successfully evaluated. */
  success: boolean;
}

export const api = {
  /**
   * Evaluates a third-party captcha client token against the gateway and returns a proof voucher.
   *
   * @param providerId - The unique identifier of the active captcha provider (e.g., "turnstile").
   * @param token - The raw response payload token received from the frontend captcha widget.
   * @returns A promise resolving to the short-lived cryptographic captcha voucher string.
   * @throws {Error} If bot behavior is detected or the captcha challenge validation fails.
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
   * Registers a new local account using an email address, password, and captcha voucher.
   *
   * @param email - Target email address for the new account.
   * @param password - Plaintext account registration password.
   * @param captchaVoucher - Proof-of-humanity voucher header string returned by `verifyCaptcha`.
   * @returns A promise that resolves upon successful registration staging.
   * @throws {Error} If the registration fails due to duplicate email or invalid payload data.
   */
  async registerLocal(email: string, password: string, captchaVoucher: string): Promise<void> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/register`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-captcha-voucher': captchaVoucher,
      },
      body: JSON.stringify({ email, password }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Failed to register account');
    }
  },

  /**
   * Authenticates a user using email and password credentials protected by captcha verification.
   *
   * @param email - The registered account email address.
   * @param password - The plaintext password attempt.
   * @param captchaVoucher - Proof-of-humanity voucher header string returned by `verifyCaptcha`.
   * @returns A promise resolving to the stateful session token string.
   * @throws {Error} If credentials are invalid, captcha proof is rejected, or login fails.
   */
  async loginLocal(email: string, password: string, captchaVoucher: string): Promise<string> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/login`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-captcha-voucher': captchaVoucher,
      },
      body: JSON.stringify({ email, password }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Failed to authenticate');
    }
    const data = await res.json();
    return data.token;
  },

  /**
   * Authenticates or provisions a user using an OAuth provider ID token protected by captcha verification.
   *
   * @param provider - OAuth identity provider identifier (e.g., "google").
   * @param idToken - Provider-issued ID token credential string.
   * @param captchaVoucher - Proof-of-humanity voucher header string returned by `verifyCaptcha`.
   * @returns A promise resolving to the stateful session token string.
   * @throws {Error} If provider authentication fails or captcha proof is rejected.
   */
  async loginOAuth(provider: string, idToken: string, captchaVoucher: string): Promise<string> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/oauth`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-captcha-voucher': captchaVoucher,
      },
      body: JSON.stringify({ provider, id_token: idToken }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'OAuth authentication failed');
    }
    const data = await res.json();
    return data.token;
  },

  /**
   * Exchanges a stateful bearer session token for a short-lived scoped access JWT.
   *
   * Automatically terminates the local session if the underlying account is marked as suspended.
   *
   * @param sessionToken - The active session token issued upon authentication.
   * @returns A promise resolving to the minted access JWT token and its expiration epoch.
   * @throws {Error} "ACCOUNT_SUSPENDED" if suspended, "ACCOUNT_UNVERIFIED" if unverified, or general failure.
   */
  async mintAccessToken(sessionToken: string): Promise<AccessTokenResponse> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/access-tokens`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({ roles: ['user'], ttl_seconds: 900 }),
    });

    if (!res.ok) {
      const errorData = await res.json().catch(() => ({}));
      if (errorData.error === 'ACCOUNT_SUSPENDED') {
        await this.logout(sessionToken).catch(() => {});
        throw new Error('ACCOUNT_SUSPENDED');
      }
      if (errorData.error === 'ACCOUNT_UNVERIFIED') {
        throw new Error('ACCOUNT_UNVERIFIED');
      }
      throw new Error('Failed to mint access token');
    }
    return res.json();
  },

  /**
   * Retrieves the current email configuration and verification state for the session owner.
   *
   * @param sessionToken - Active session token used for authorization header.
   * @returns A promise resolving to the email configuration state object.
   * @throws {Error} If fetching state fails or authentication token is invalid.
   */
  async getEmailState(sessionToken: string): Promise<EmailStateResponse> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/email`, {
      headers: { Authorization: `Bearer ${sessionToken}` },
    });
    if (!res.ok) throw new Error('Failed to fetch email state');
    return res.json();
  },

  /**
   * Initiates an email destination change request protected by captcha verification.
   *
   * @param sessionToken - Active session token for user identification.
   * @param email - Target email address to set or stage.
   * @param captchaVoucher - Proof-of-humanity voucher header string.
   * @returns A promise resolving to the status of the state machine change.
   * @throws {Error} If updating the email fails or captcha validation is rejected.
   */
  async setEmail(
    sessionToken: string,
    email: string,
    captchaVoucher: string
  ): Promise<SetEmailResponse> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/email`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${sessionToken}`,
        'x-captcha-voucher': captchaVoucher,
      },
      body: JSON.stringify({ email }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Failed to update email');
    }
    return res.json();
  },

  /**
   * Evaluates a numeric/alphanumeric challenge verification code without requiring an auth header.
   *
   * @param userId - Target user identifier (UUID) owning the email challenge.
   * @param email - Target unverified email address undergoing challenge verification.
   * @param code - 6-digit challenge verification code string dispatched to user inbox.
   * @returns A promise resolving to the verification result boolean wrapper.
   * @throws {Error} If the code is invalid, expired, or payload parsing fails.
   */
  async verifyEmailPublic(
    userId: string,
    email: string,
    code: string
  ): Promise<VerifyEmailResponse> {
    const res = await fetch(`${GATEWAY_URL}/api/v1/email/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ user_id: userId, email, code }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Invalid or expired verification code');
    }
    return res.json();
  },

  /**
   * Revokes the provided active session token on the gateway.
   *
   * @param sessionToken - The active session token to invalidate.
   * @returns A promise that resolves when the logout request finishes.
   */
  async logout(sessionToken: string): Promise<void> {
    await fetch(`${GATEWAY_URL}/api/v1/logout`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${sessionToken}` },
    });
  },
};
