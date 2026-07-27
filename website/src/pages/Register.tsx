//! User registration view with Turnstile CAPTCHA verification and Google OAuth.

import { Turnstile } from '@marsidev/react-turnstile';
import { GoogleLogin, type CredentialResponse } from '@react-oauth/google';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { GATEWAY_URL, TURNSTILE_SITE_KEY } from '../config.js';
import { api } from '../lib/api.js';

/**
 * User registration form component.
 * Handles account creation by validating credentials and requiring a completed Turnstile CAPTCHA voucher before submission.
 */
export default function Register() {
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [captchaVoucher, setCaptchaVoucher] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  /**
   * Handles Turnstile client completion, exchanging the challenge token for a backend voucher.
   *
   * @param token - Raw Turnstile challenge completion token.
   */
  const handleTurnstileSuccess = async (token: string) => {
    try {
      const voucher = await api.verifyCaptcha('turnstile', token);
      setCaptchaVoucher(voucher);
      setError(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Turnstile verification failed');
    }
  };

  /**
   * Exchanges a raw session token for an access JWT, persists auth tokens to local storage, and redirects to the dashboard.
   *
   * @param sessionToken - Raw gateway session token returned from authentication endpoints.
   */
  const processSession = async (sessionToken: string) => {
    const jwtData = await api.mintAccessToken(sessionToken);
    localStorage.setItem('your_app_session', sessionToken);
    localStorage.setItem('your_app_jwt', jwtData.access_token);
    navigate('/dashboard');
  };

  /**
   * Submits traditional registration credentials along with the CAPTCHA voucher to create an account.
   *
   * @param e - Form submit event.
   */
  const handleLocalRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!captchaVoucher) return setError('Please complete the human verification check.');

    try {
      const res = await fetch(`${GATEWAY_URL}/api/v1/register`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-captcha-voucher': captchaVoucher,
        },
        body: JSON.stringify({ email, password }),
      });

      const data = await res.json();
      if (!res.ok) throw new Error(data.error || 'Registration failed');

      navigate('/login', {
        state: { message: 'Account created! Please check your email for the verification code.' },
      });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Registration failed');
    }
  };

  /**
   * Authenticates and automatically registers using a Google OAuth ID credential.
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
      if (!res.ok) throw new Error(data.error || 'OAuth Registration failed');

      // OAuth users are implicitly verified by their provider, so we skip the email code and log them right in.
      await processSession(data.token);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'OAuth Registration failed');
    }
  };

  return (
    <div style={{ maxWidth: '400px', margin: '50px auto', fontFamily: 'sans-serif' }}>
      <h1>Create YourApp Account</h1>
      {error && <p style={{ color: 'red' }}>{error}</p>}

      {TURNSTILE_SITE_KEY && (
        <div style={{ marginBottom: '20px' }}>
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />
        </div>
      )}

      {captchaVoucher && (
        <>
          <form
            onSubmit={handleLocalRegister}
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

            <button type="submit">Register</button>
          </form>

          <hr style={{ margin: '20px 0' }} />

          <GoogleLogin
            text="signup_with"
            onSuccess={handleGoogleSuccess}
            onError={() => setError('Google OAuth Failed')}
          />
        </>
      )}
    </div>
  );
}
