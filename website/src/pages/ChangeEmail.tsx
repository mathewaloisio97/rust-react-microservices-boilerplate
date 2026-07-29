/**
 * Email change management and verification component.
 *
 * Handles two primary UI phases driven by the backend state machine:
 * 1. **Initial Email Change Request (`verification_type == null`):**
 *    Allows the user to input a new email address. Protected by Turnstile captcha verification.
 * 2. **Multi-Step Email Verification Pipeline (`CONFIRM_OLD` / `CONFIRM_NEW`):**
 *    - `CONFIRM_OLD`: User inputs a 6-digit verification code sent to their current email address.
 *    - `CONFIRM_NEW`: User inputs a 6-digit verification code sent to their pending new email address.
 *
 * Behavioral Optimizations:
 * - Allows instant input of the new email address while Turnstile initializes asynchronously in the background.
 * - Auto-executes the change request if the user clicks "Request Change" before Turnstile completes.
 * - Code verification steps do not require Turnstile challenges.
 *
 * @module pages/ChangeEmail
 */

import { Turnstile } from '@marsidev/react-turnstile';
import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { TURNSTILE_SITE_KEY } from '../config.js';
import { api, type EmailStateResponse } from '../lib/api.js';

/**
 * Account email management view component.
 */
export function ChangeEmail() {
  const navigate = useNavigate();

  /** Active email configuration and state-machine status. */
  const [emailState, setEmailState] = useState<EmailStateResponse | null>(null);

  /** Unblocked text input state for new target email address. */
  const [newEmail, setNewEmail] = useState('');

  /** Unblocked text input state for 6-digit challenge verification codes. */
  const [code, setCode] = useState('');

  /** Raw Turnstile challenge token generated upon widget completion. */
  const [captcha, setCaptcha] = useState('');

  /** UI error banner string. */
  const [error, setError] = useState('');

  /** Async API loading indicator. */
  const [loading, setLoading] = useState(false);

  /** Tracks if user submitted an email change request prior to Turnstile resolution. */
  const [awaitingCaptcha, setAwaitingCaptcha] = useState(false);

  /** Ref storing target email address for deferred auto-submission. */
  const submittedEmailRef = useRef('');

  /**
   * Initializes email state on mount. Redirects to `/login` if session token is missing.
   */
  useEffect(() => {
    const st = localStorage.getItem('sessionToken');
    if (!st) {
      navigate('/login');
      return;
    }

    fetchState(st);
  }, [navigate]);

  /**
   * Fetches the current email state machine context from the API Gateway.
   *
   * @param st - Active session token.
   */
  const fetchState = async (st: string) => {
    try {
      const state = await api.getEmailState(st);
      setEmailState(state);
    } catch {
      navigate('/login');
    }
  };

  /**
   * Executes the email change request API call once a valid Turnstile token is available.
   *
   * @param targetEmail - Desired new email address.
   * @param captchaToken - Valid Turnstile verification token.
   */
  const executeRequestChange = async (targetEmail: string, captchaToken: string) => {
    setLoading(true);
    setError('');

    try {
      const st = localStorage.getItem('sessionToken')!;
      const voucher = await api.verifyCaptcha('turnstile', captchaToken);
      await api.setEmail(st, targetEmail, voucher);

      // If the account was PENDING/UNVERIFIED, setEmail instantly updates it and resends code
      if (!emailState?.is_verified) {
        navigate('/verify');
      } else {
        // Active verified account needs to follow the multi-step verification pipeline
        await fetchState(st);
        setCaptcha('');
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to request email change');
    } finally {
      setLoading(false);
      setAwaitingCaptcha(false);
    }
  };

  /**
   * Callback fired upon Cloudflare Turnstile challenge completion.
   * Automatically triggers email change execution if the user submitted early.
   *
   * @param token - Issued Turnstile challenge token string.
   */
  const handleTurnstileSuccess = (token: string) => {
    setCaptcha(token);

    if (awaitingCaptcha) {
      executeRequestChange(submittedEmailRef.current, token);
    }
  };

  /**
   * Handles submission of a new email change request.
   *
   * @param e - React form submit event.
   */
  const handleRequestChange = (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    if (!newEmail) {
      return setError('Please enter a new email address.');
    }

    submittedEmailRef.current = newEmail;

    if (!captcha) {
      setAwaitingCaptcha(true);
      return;
    }

    executeRequestChange(newEmail, captcha);
  };

  /**
   * Handles submission of challenge verification codes during pipeline stages (`CONFIRM_OLD` / `CONFIRM_NEW`).
   *
   * @param e - React form submit event.
   */
  const handleVerifyStep = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!emailState) return;

    setError('');
    setLoading(true);

    const st = localStorage.getItem('sessionToken')!;
    const targetEmail =
      emailState.verification_type === 'CONFIRM_OLD'
        ? emailState.current_email
        : emailState.pending_new_email;

    try {
      await api.verifyEmailPublic(emailState.user_id, targetEmail, code);
      setCode('');
      const newState = await api.getEmailState(st);

      if (newState.is_verified && !newState.verification_type) {
        // Pipeline successfully completed; return to main workspace
        navigate('/dashboard');
      } else {
        setEmailState(newState);
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Invalid or expired verification code');
    } finally {
      setLoading(false);
    }
  };

  if (!emailState) return <p style={{ padding: '2rem' }}>Loading data...</p>;

  // Stage 1: Request Email Change
  if (!emailState.verification_type) {
    return (
      <div style={{ maxWidth: '400px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
        <h1>Change Email</h1>
        {emailState.is_verified && (
          <p>
            Current Email: <strong>{emailState.current_email}</strong>
          </p>
        )}
        {error && <div style={{ color: 'red', marginBottom: '1rem' }}>{error}</div>}

        <form
          onSubmit={handleRequestChange}
          style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}
        >
          <input
            type="email"
            placeholder="New Email Address"
            value={newEmail}
            onChange={(e) => setNewEmail(e.target.value)}
            disabled={loading}
            required
          />

          {/* Turnstile resolves asynchronously without blocking text entry */}
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />

          <button type="submit" disabled={loading}>
            {loading
              ? 'Requesting...'
              : awaitingCaptcha
                ? 'Verifying security check...'
                : 'Request Change'}
          </button>
        </form>
      </div>
    );
  }

  // Stage 2 & 3: Challenge Code Verification (CONFIRM_OLD or CONFIRM_NEW)
  const isConfirmingOld = emailState.verification_type === 'CONFIRM_OLD';
  const displayEmail = isConfirmingOld ? emailState.current_email : emailState.pending_new_email;
  const stepText = isConfirmingOld
    ? 'To authorize this change, please enter the code sent to your old address'
    : 'To finalize the change, enter the code sent to your new address';

  return (
    <div style={{ maxWidth: '400px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
      <h1>Security Verification</h1>
      <p>
        {stepText} (<strong>{displayEmail}</strong>).
      </p>

      {error && <div style={{ color: 'red', marginBottom: '1rem' }}>{error}</div>}

      <form
        onSubmit={handleVerifyStep}
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
          {loading ? 'Verifying...' : 'Verify'}
        </button>
      </form>
    </div>
  );
}
