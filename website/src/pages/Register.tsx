/**
 * User registration view component with Turnstile verification and Google OAuth.
 *
 * Key Behavioral Optimizations:
 * - Allows users to immediately enter local registration credentials while Turnstile loads in parallel.
 * - Auto-executes local registration once Turnstile succeeds if the user clicked "Register" early.
 * - Renders Google's official `<GoogleLogin />` SDK widget once human verification completes.
 * - Handles auto-provisioning and session token processing for OAuth users.
 *
 * @module pages/Register
 */

import { Turnstile } from '@marsidev/react-turnstile';
import { GoogleLogin, type CredentialResponse } from '@react-oauth/google';
import { useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { TURNSTILE_SITE_KEY } from '../config.js';
import { api } from '../lib/api.js';

/**
 * Primary user registration view component.
 *
 * @returns The rendered user registration form and OAuth provider interface.
 */
export function Register() {
  /** Target email input state (immediately editable). */
  const [email, setEmail] = useState('');

  /** Target password input state (immediately editable). */
  const [password, setPassword] = useState('');

  /** Captcha voucher issued by backend after Turnstile token verification. */
  const [captchaVoucher, setCaptchaVoucher] = useState<string | null>(null);

  /** UI error message string. */
  const [error, setError] = useState<string | null>(null);

  /** Active API request loading state. */
  const [loading, setLoading] = useState(false);

  /** Tracks whether the user clicked "Register" before Turnstile finished. */
  const [awaitingCaptcha, setAwaitingCaptcha] = useState(false);

  /** React Router navigation hook. */
  const navigate = useNavigate();

  /** Ref tracking submitted registration credentials for deferred auto-execution. */
  const submittedDataRef = useRef({ email: '', password: '' });

  /**
   * Exchanges a raw session token for an access JWT, persists auth tokens, and redirects to dashboard.
   *
   * @param sessionToken - Raw gateway session token returned from OAuth authentication.
   */
  const processSession = async (sessionToken: string) => {
    const jwtData = await api.mintAccessToken(sessionToken);
    localStorage.setItem('sessionToken', sessionToken);
    localStorage.setItem('accessToken', jwtData.access_token);
    navigate('/dashboard');
  };

  /**
   * Core local registration pipeline execution using a valid captcha voucher.
   *
   * @param targetEmail - User email address.
   * @param targetPass - Plaintext password.
   * @param voucher - Verified captcha voucher token string.
   */
  const executeRegister = async (targetEmail: string, targetPass: string, voucher: string) => {
    setLoading(true);
    setError(null);
    try {
      await api.registerLocal(targetEmail, targetPass, voucher);
      alert('Registration successful! Check your inbox for the verification link.');
      navigate('/login');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Registration failed');
    } finally {
      setLoading(false);
      setAwaitingCaptcha(false);
    }
  };

  /**
   * Handles Turnstile client completion, exchanging the raw challenge token for a backend voucher.
   * If the user clicked "Register" early, automatically triggers registration execution.
   *
   * @param token - Raw Turnstile challenge completion token.
   */
  const handleTurnstileSuccess = async (token: string) => {
    try {
      const voucher = await api.verifyCaptcha('turnstile', token);
      setCaptchaVoucher(voucher);
      setError(null);

      if (awaitingCaptcha) {
        const { email: pendingEmail, password: pendingPassword } = submittedDataRef.current;
        await executeRegister(pendingEmail, pendingPassword, voucher);
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Turnstile verification failed');
      setAwaitingCaptcha(false);
    }
  };

  /**
   * Handles local form submission.
   *
   * @param e - Form submit event.
   */
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!email || !password) {
      return setError('Please provide both an email and a password.');
    }

    if (password.length < 8) {
      return setError('Password must be at least 8 characters long.');
    }

    submittedDataRef.current = { email, password };

    if (!captchaVoucher) {
      setAwaitingCaptcha(true);
      return;
    }

    executeRegister(email, password, captchaVoucher);
  };

  /**
   * Authenticates and automatically registers using a Google OAuth ID credential.
   *
   * @param credentialResponse - Response object from Google OAuth widget containing the ID token.
   */
  const handleGoogleSuccess = async (credentialResponse: CredentialResponse) => {
    if (!captchaVoucher) return setError('Please complete human verification.');

    if (!credentialResponse.credential) {
      return setError('Google OAuth failed: No credential returned.');
    }

    setLoading(true);
    setError(null);
    try {
      const sessionToken = await api.loginOAuth(
        'google',
        credentialResponse.credential,
        captchaVoucher
      );
      await processSession(sessionToken);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'OAuth Registration failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ maxWidth: '400px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
      <h1>Create YourApp Account</h1>
      {error && <div style={{ color: 'red', marginBottom: '1rem' }}>{error}</div>}

      <form
        onSubmit={handleSubmit}
        style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}
      >
        <input
          type="email"
          placeholder="Email Address"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          disabled={loading}
          required
        />
        <input
          type="password"
          placeholder="Password (min 8 characters)"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={loading}
          required
          minLength={8}
        />

        {TURNSTILE_SITE_KEY && (
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />
        )}

        <button type="submit" disabled={loading}>
          {loading ? 'Creating...' : awaitingCaptcha ? 'Verifying security check...' : 'Register'}
        </button>
      </form>

      <div style={{ margin: '1.5rem 0', textAlign: 'center', color: '#666' }}>OR</div>

      {/* Google OAuth Section (Rendered when Captcha is Verified) */}
      <div style={{ display: 'flex', justifyContent: 'center' }}>
        {captchaVoucher ? (
          <GoogleLogin
            text="signup_with"
            onSuccess={handleGoogleSuccess}
            onError={() => setError('Google OAuth Failed')}
          />
        ) : (
          <p style={{ color: '#666', fontSize: '0.9rem' }}>
            Complete the security check above to enable Google Sign-Up.
          </p>
        )}
      </div>

      <p style={{ marginTop: '1.5rem' }}>
        Already have an account? <Link to="/login">Log In</Link>
      </p>
    </div>
  );
}
