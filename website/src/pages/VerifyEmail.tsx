/**
 * Email verification view component.
 *
 * Supports two operational modes:
 * 1. **Public Verification Link Mode:** Triggered via URL search parameters (`?code=...&email=...&user_id=...`)
 *    when a user clicks a verification link in their email inbox.
 * 2. **Authenticated Code Entry Mode:** Rendered when a logged-in user with an unverified account manually inputs
 *    their 6-digit challenge code or requests a fresh verification email dispatch.
 *
 * Behavioral Optimizations:
 * - Manual 6-digit code entry does not require Turnstile verification.
 * - Resend verification requests run Turnstile in parallel and auto-execute if the user confirms early.
 * - Allows logging out cleanly from the verification state.
 *
 * @module pages/VerifyEmail
 */

import { Turnstile } from '@marsidev/react-turnstile';
import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { TURNSTILE_SITE_KEY } from '../config.js';
import { api, type EmailStateResponse } from '../lib/api.js';

/**
 * Email verification page component.
 */
export function VerifyEmail() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();

  /** Query string parameters extracted for incoming verification links. */
  const urlCode = searchParams.get('code');
  const urlEmail = searchParams.get('email');
  const urlUserId = searchParams.get('user_id');

  /** Manual 6-digit challenge code input state. */
  const [code, setCode] = useState('');

  /** UI error message banner string. */
  const [error, setError] = useState('');

  /** UI success/info notification message string. */
  const [msg, setMsg] = useState('');

  /** Active email configuration state for logged-in users. */
  const [emailState, setEmailState] = useState<EmailStateResponse | null>(null);

  /** Raw Turnstile challenge token string for resend operations. */
  const [captcha, setCaptcha] = useState('');

  /** Async API request loading indicator. */
  const [loading, setLoading] = useState(false);

  /** Controls Turnstile widget visibility during email resend requests. */
  const [showTurnstile, setShowTurnstile] = useState(false);

  /** Tracks if user confirmed a resend while Turnstile was still resolving. */
  const [awaitingCaptcha, setAwaitingCaptcha] = useState(false);

  /** Ref tracking active Turnstile token for deferred resend triggers. */
  const captchaRef = useRef('');

  // Synchronize ref with state
  captchaRef.current = captcha;

  /**
   * Clears session tokens from storage and redirects the user to the login screen.
   */
  const handleLogout = () => {
    localStorage.removeItem('sessionToken');
    localStorage.removeItem('accessToken');
    navigate('/login');
  };

  /**
   * Initializes email verification state on component mount.
   * Handles incoming query-string verification links or fetches active session email state.
   */
  useEffect(() => {
    // Mode 1: Incoming verification link click from inbox
    if (urlCode && urlEmail && urlUserId) {
      setLoading(true);
      api
        .verifyEmailPublic(urlUserId, urlEmail, urlCode)
        .then(() => {
          setMsg('Email verified successfully! You can now log in.');
          setTimeout(() => navigate('/login'), 3000);
        })
        .catch((e: unknown) => {
          setError(e instanceof Error ? e.message : 'Verification failed');
          setLoading(false);
        });
      return;
    }

    // Mode 2: Logged-in user lacking verification
    const st = localStorage.getItem('sessionToken');
    if (!st) {
      navigate('/login');
      return;
    }

    api
      .getEmailState(st)
      .then((state) => {
        if (state.is_verified) {
          navigate('/dashboard');
        } else {
          setEmailState(state);
        }
      })
      .catch(() => {
        localStorage.removeItem('sessionToken');
        navigate('/login');
      });
  }, [urlCode, urlEmail, urlUserId, navigate]);

  /**
   * Evaluates manual 6-digit code entry against the public verification gateway.
   * On success, mints an access JWT and redirects the user to `/dashboard`.
   *
   * @param e - React form submit event.
   */
  const handleManualSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!emailState) return;

    setError('');
    setMsg('');
    setLoading(true);

    try {
      await api.verifyEmailPublic(emailState.user_id, emailState.current_email, code);

      // Verification passed; mint active dashboard access token
      const st = localStorage.getItem('sessionToken')!;
      const { access_token } = await api.mintAccessToken(st);
      localStorage.setItem('accessToken', access_token);
      navigate('/dashboard');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Invalid or expired verification code');
      setLoading(false);
    }
  };

  /**
   * Executes the resend email API request once Turnstile is resolved.
   *
   * @param captchaToken - Valid Turnstile verification token.
   */
  const executeResend = async (captchaToken: string) => {
    if (!emailState) return;

    setLoading(true);
    setError('');
    setMsg('');

    try {
      const st = localStorage.getItem('sessionToken')!;
      const voucher = await api.verifyCaptcha('turnstile', captchaToken);
      await api.setEmail(st, emailState.current_email, voucher);

      setMsg('A fresh verification code has been dispatched to your inbox.');
      setShowTurnstile(false);
      setCaptcha('');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to resend verification email');
    } finally {
      setLoading(false);
      setAwaitingCaptcha(false);
    }
  };

  /**
   * Callback fired upon Turnstile challenge resolution during resend flow.
   * Auto-triggers resend if user clicked confirm early.
   *
   * @param token - Issued Turnstile challenge token string.
   */
  const handleTurnstileSuccess = (token: string) => {
    setCaptcha(token);

    if (awaitingCaptcha) {
      executeResend(token);
    }
  };

  /**
   * Handles user interaction for requesting a fresh email verification code.
   */
  const handleResend = () => {
    if (!showTurnstile) {
      setShowTurnstile(true);
      return;
    }

    if (!captcha) {
      setAwaitingCaptcha(true);
      return;
    }

    executeResend(captcha);
  };

  // Render view for Mode 1 (Incoming URL Magic Link)
  if (urlCode) {
    return (
      <div style={{ maxWidth: '500px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
        <h2>Verifying Email...</h2>
        {error ? <p style={{ color: 'red' }}>{error}</p> : <p style={{ color: 'green' }}>{msg}</p>}
        {error && <Link to="/login">Return to login</Link>}
      </div>
    );
  }

  if (!emailState) return <p style={{ padding: '2rem' }}>Loading verification data...</p>;

  // Render view for Mode 2 (Authenticated Manual Code Entry)
  return (
    <div style={{ maxWidth: '400px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1>Verify Email</h1>
        <button
          type="button"
          onClick={handleLogout}
          style={{
            background: 'transparent',
            border: '1px solid #ccc',
            borderRadius: '4px',
            padding: '0.25rem 0.75rem',
            cursor: 'pointer',
            fontSize: '0.875rem',
          }}
        >
          Log Out
        </button>
      </div>

      <p>
        A verification code was sent to <strong>{emailState.current_email}</strong>.
      </p>

      {error && <div style={{ color: 'red', marginBottom: '1rem' }}>{error}</div>}
      {msg && <div style={{ color: 'green', marginBottom: '1rem' }}>{msg}</div>}

      <form
        onSubmit={handleManualSubmit}
        style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}
      >
        <input
          type="text"
          placeholder="6-Digit Code"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          disabled={loading}
          required
          maxLength={6}
        />
        <button type="submit" disabled={loading}>
          {loading ? 'Submitting...' : 'Submit Code'}
        </button>
      </form>

      <div style={{ marginTop: '2rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <button
          type="button"
          onClick={handleResend}
          disabled={loading}
          style={{ background: '#eee', color: '#333' }}
        >
          {loading
            ? 'Processing...'
            : awaitingCaptcha
              ? 'Verifying security check...'
              : showTurnstile
                ? 'Confirm Resend Request'
                : 'Resend Verification Email'}
        </button>

        {showTurnstile && (
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />
        )}

        <button
          type="button"
          onClick={() => navigate('/change-email')}
          disabled={loading}
          style={{
            background: 'transparent',
            color: 'blue',
            border: 'none',
            textDecoration: 'underline',
            cursor: 'pointer',
          }}
        >
          Change Email Address
        </button>
      </div>
    </div>
  );
}
