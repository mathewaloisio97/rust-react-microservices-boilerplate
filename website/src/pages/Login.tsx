//! Authentication view supporting email/password and Google OAuth login with CAPTCHA verification.

import { Turnstile } from '@marsidev/react-turnstile';
import { GoogleLogin, type CredentialResponse } from '@react-oauth/google';
import { useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { GATEWAY_URL, TURNSTILE_SITE_KEY } from '../config.js';
import { api } from '../lib/api.js';

/**
 * User login interface enabling standard and social authentication once a Turnstile CAPTCHA is verified.
 */
export default function Login() {
  const location = useLocation();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [captchaVoucher, setCaptchaVoucher] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  /**
   * Evaluates successful Turnstile completion and stores the gateway CAPTCHA voucher.
   *
   * @param token - The client-side Turnstile challenge response token.
   */
  const handleTurnstileSuccess = async (token: string) => {
    try {
      const voucher = await api.verifyCaptcha('turnstile', token);
      setCaptchaVoucher(voucher);
      setError(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Turnstile failed');
    }
  };

  /**
   * Exchanges a raw session token for an access JWT, persists auth tokens to local storage, and redirects to the dashboard.
   *
   * @param sessionToken - Raw gateway session token returned from authentication endpoints.
   */
  const processSession = async (sessionToken: string) => {
    const jwtData = await api.mintAccessToken(sessionToken);
    localStorage.setItem('cleard_session', sessionToken);
    localStorage.setItem('cleard_jwt', jwtData.access_token);
    navigate('/dashboard');
  };

  /**
   * Authenticates using traditional email and password credentials.
   *
   * @param e - Form submit event.
   */
  const handleLocalLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!captchaVoucher) return setError('Please complete human verification.');

    try {
      const res = await fetch(`${GATEWAY_URL}/api/v1/login`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-captcha-voucher': captchaVoucher,
        },
        body: JSON.stringify({ email, password }),
      });

      const data = await res.json();
      if (!res.ok) throw new Error(data.error || 'Login failed');

      await processSession(data.token);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Login failed');
    }
  };

  /**
   * Authenticates using a Google OAuth ID credential.
   *
   * @param credentialResponse - Response object returned from Google OAuth trigger containing the ID token.
   */
  const handleGoogleSuccess = async (credentialResponse: CredentialResponse) => {
    if (!captchaVoucher) return setError('Please complete human verification.');

    if (!credentialResponse.credential) {
      return setError('Google OAuth failed: No credential returned.');
    }

    try {
      const res = await fetch(`${GATEWAY_URL}/api/v1/oauth`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-captcha-voucher': captchaVoucher,
        },
        body: JSON.stringify({ provider: 'google', id_token: credentialResponse.credential }),
      });

      const data = await res.json();
      if (!res.ok) throw new Error(data.error || 'OAuth Login failed');

      await processSession(data.token);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'OAuth Login failed');
    }
  };

  return (
    <div style={{ maxWidth: '400px', margin: '50px auto', fontFamily: 'sans-serif' }}>
      <h1>Login to Cleard</h1>
      {location.state?.message && <p style={{ color: 'green' }}>{location.state.message}</p>}
      {error && <p style={{ color: 'red' }}>{error}</p>}

      {TURNSTILE_SITE_KEY && (
        <div style={{ marginBottom: '20px' }}>
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />
        </div>
      )}

      {captchaVoucher && (
        <>
          <form
            onSubmit={handleLocalLogin}
            style={{ display: 'flex', flexDirection: 'column', gap: '15px', marginBottom: '20px' }}
          >
            <input
              type="email"
              placeholder="Email Address"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
            <input
              type="password"
              placeholder="Password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
            <button type="submit">Login</button>
          </form>

          <hr style={{ margin: '20px 0' }} />

          <GoogleLogin
            onSuccess={handleGoogleSuccess}
            onError={() => setError('Google OAuth Failed')}
          />
        </>
      )}
    </div>
  );
}
