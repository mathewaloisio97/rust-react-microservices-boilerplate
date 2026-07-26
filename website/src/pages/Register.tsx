//! User registration view with Turnstile CAPTCHA verification.

import { Turnstile } from '@marsidev/react-turnstile';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../lib/api.js';

const GATEWAY_URL = import.meta.env.VITE_GATEWAY_URL || 'http://localhost:3000';
const TURNSTILE_SITE_KEY = import.meta.env.VITE_TURNSTILE_SITE_KEY || '';

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
   * Submits registration credentials along with the CAPTCHA voucher to create an account.
   *
   * @param e - Form submit event.
   */
  const handleRegister = async (e: React.FormEvent) => {
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

  return (
    <div style={{ maxWidth: '400px', margin: '50px auto', fontFamily: 'sans-serif' }}>
      <h1>Create Cleard Account</h1>
      {error && <p style={{ color: 'red' }}>{error}</p>}

      <form
        onSubmit={handleRegister}
        style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}
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

        {TURNSTILE_SITE_KEY && (
          <Turnstile siteKey={TURNSTILE_SITE_KEY} onSuccess={handleTurnstileSuccess} />
        )}

        <button type="submit" disabled={!captchaVoucher}>
          Register
        </button>
      </form>
    </div>
  );
}
