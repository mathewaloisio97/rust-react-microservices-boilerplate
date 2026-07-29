/**
 * User authentication and login view component with Turnstile verification and Google OAuth.
 *
 * Key Behavioral Optimizations:
 * - Unblocked form inputs allowing immediate credential entry while Turnstile loads.
 * - Auto-executes local authentication once Turnstile completes if submitted early.
 * - Renders Google's official `<GoogleLogin />` SDK widget once human verification completes.
 * - Handles gRPC status redirects (`ACCOUNT_UNVERIFIED`, `ACCOUNT_SUSPENDED`) cleanly.
 *
 * @module pages/Login
 */

import { Turnstile } from '@marsidev/react-turnstile';
import { GoogleLogin, type CredentialResponse } from '@react-oauth/google';
import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { TURNSTILE_SITE_KEY } from '../config.js';
import { api } from '../lib/api.js';

/**
 * Primary user login view component.
 *
 * @returns The rendered user login form and OAuth provider interface.
 */
export function Login() {
  /** User email input state. */
  const [email, setEmail] = useState('');

  /** User password input state. */
  const [password, setPassword] = useState('');

  /** Captcha voucher issued by backend after Turnstile token verification. */
  const [captchaVoucher, setCaptchaVoucher] = useState<string | null>(null);

  /** UI error message string. */
  const [error, setError] = useState<string | null>(null);

  /** Active API request loading state. */
  const [loading, setLoading] = useState(false);

  /** Tracks whether the user clicked "Sign In" before Turnstile finished. */
  const [awaitingCaptcha, setAwaitingCaptcha] = useState(false);

  /** React Router navigation hook. */
  const navigate = useNavigate();

  /** Ref tracking submitted credentials for deferred auto-execution. */
  const submittedDataRef = useRef({ email: '', password: '' });

  /**
   * Redirects authenticated users directly to the dashboard.
   */
  useEffect(() => {
    if (localStorage.getItem('sessionToken')) {
      navigate('/dashboard', { replace: true });
    }
  }, [navigate]);

  /**
   * Core login execution pipeline. Exchanges local credentials and voucher for session tokens.
   *
   * @param targetEmail - User email address.
   * @param targetPass - Plaintext password.
   * @param voucher - Verified captcha voucher token string.
   */
  const executeLogin = async (targetEmail: string, targetPass: string, voucher: string) => {
    setLoading(true);
    setError(null);
    try {
      const sessionToken = await api.loginLocal(targetEmail, targetPass, voucher);
      localStorage.setItem('sessionToken', sessionToken);

      try {
        const { access_token } = await api.mintAccessToken(sessionToken);
        localStorage.setItem('accessToken', access_token);
        navigate('/dashboard');
      } catch (mintErr: unknown) {
        const msg = mintErr instanceof Error ? mintErr.message : String(mintErr);
        if (msg === 'ACCOUNT_UNVERIFIED') {
          navigate('/verify');
        } else if (msg === 'ACCOUNT_SUSPENDED') {
          navigate('/suspended');
        } else {
          setError(msg);
        }
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Authentication failed');
    } finally {
      setLoading(false);
      setAwaitingCaptcha(false);
    }
  };

  /**
   * Handles Turnstile client completion, exchanging raw challenge token for backend voucher.
   * Auto-triggers pending login request if user clicked "Sign In" early.
   *
   * @param token - Raw Turnstile challenge token.
   */
  const handleTurnstileSuccess = async (token: string) => {
    try {
      const voucher = await api.verifyCaptcha('turnstile', token);
      setCaptchaVoucher(voucher);
      setError(null);

      if (awaitingCaptcha) {
        const { email: pendingEmail, password: pendingPassword } = submittedDataRef.current;
        await executeLogin(pendingEmail, pendingPassword, voucher);
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
      return setError('Please enter both email and password.');
    }

    submittedDataRef.current = { email, password };

    if (!captchaVoucher) {
      setAwaitingCaptcha(true);
      return;
    }

    executeLogin(email, password, captchaVoucher);
  };

  /**
   * Authenticates using a Google OAuth ID credential.
   *
   * @param credentialResponse - Response object from Google OAuth widget.
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
      localStorage.setItem('sessionToken', sessionToken);

      const { access_token } = await api.mintAccessToken(sessionToken);
      localStorage.setItem('accessToken', access_token);
      navigate('/dashboard');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Google login failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ maxWidth: '400px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
      <h1>Log In</h1>
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
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={loading}
          required
        />

        {TURNSTILE_SITE_KEY && (
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />
        )}

        <button type="submit" disabled={loading}>
          {loading ? 'Logging in...' : awaitingCaptcha ? 'Verifying security check...' : 'Sign In'}
        </button>
      </form>

      <div style={{ margin: '1.5rem 0', textAlign: 'center', color: '#666' }}>OR</div>

      {/* Google OAuth Section */}
      <div style={{ display: 'flex', justifyContent: 'center' }}>
        {captchaVoucher ? (
          <GoogleLogin
            text="signin_with"
            onSuccess={handleGoogleSuccess}
            onError={() => setError('Google OAuth Failed')}
          />
        ) : (
          <p style={{ color: '#666', fontSize: '0.9rem' }}>
            Complete the security check above to enable Google Sign-In.
          </p>
        )}
      </div>

      <p style={{ marginTop: '1.5rem' }}>
        Don&apos;t have an account? <Link to="/register">Create one</Link>
      </p>
    </div>
  );
}
