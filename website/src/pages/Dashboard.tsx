/**
 * User dashboard and workspace landing component.
 *
 * Serves as the primary landing page for authenticated users with fully verified emails.
 * Performs a verification check on mount:
 * - If the email is unverified, redirects the user to `/verify`.
 * - If session is missing or invalid, clears tokens and redirects to `/login`.
 *
 * Exposes core account management actions: Conditionally rendering email settings
 * based on provider permissions and logging out.
 *
 * @module pages/Dashboard
 */

import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api, type EmailStateResponse } from '../lib/api.js';

/**
 * Formats the primary login banner string to reflect the authentication provider.
 *
 * @param email - Primary confirmed email address.
 * @param provider - Auth provider identifier (e.g. 'local', 'google', 'apple').
 * @returns Formatted status string.
 */
function formatLoginLabel(email: string, provider?: string): string {
  switch (provider?.toLowerCase()) {
    case 'google':
      return `Logged in as: ${email} (Google OAuth)`;
    case 'apple':
      return `Logged in as: ${email} (Apple SSO)`;
    case 'sso':
      return `Logged in as: ${email} (SSO Provider)`;
    case 'local':
    default:
      return `Logged in as: ${email}`;
  }
}

/**
 * Main application dashboard page component.
 */
export function Dashboard() {
  const navigate = useNavigate();

  /** Email state context for the currently logged-in user. */
  const [emailState, setEmailState] = useState<EmailStateResponse | null>(null);

  /** Async loading state while validating session and email verification. */
  const [loading, setLoading] = useState(true);

  /** Async loading state for the logout action. */
  const [loggingOut, setLoggingOut] = useState(false);

  /**
   * Verifies account status on mount.
   */
  useEffect(() => {
    const sessionToken = localStorage.getItem('sessionToken');
    if (!sessionToken) {
      navigate('/login');
      return;
    }

    api
      .getEmailState(sessionToken)
      .then((state) => {
        if (!state.is_verified) {
          // Redirect unverified users directly to the verification page.
          navigate('/verify');
        } else {
          setEmailState(state);
          setLoading(false);
        }
      })
      .catch(() => {
        // Clear invalid session tokens and redirect to login.
        localStorage.removeItem('sessionToken');
        localStorage.removeItem('accessToken');
        navigate('/login');
      });
  }, [navigate]);

  /**
   * Performs session invalidation and logs out the user.
   */
  const handleLogout = async () => {
    const sessionToken = localStorage.getItem('sessionToken');
    setLoggingOut(true);

    if (sessionToken) {
      try {
        await api.logout(sessionToken);
      } catch (err) {
        console.error('Logout error:', err);
      }
    }

    // Purge local token storage.
    localStorage.removeItem('sessionToken');
    localStorage.removeItem('accessToken');
    navigate('/login');
  };

  if (loading) {
    return <p style={{ padding: '2rem' }}>Loading workspace...</p>;
  }

  return (
    <div style={{ maxWidth: '500px', margin: '4rem auto', fontFamily: 'sans-serif' }}>
      <h1>Dashboard</h1>
      <p style={{ color: '#2e7d32', fontWeight: 'bold' }}>✓ Account Verified & Active</p>

      {emailState && (
        <p>
          <strong>{formatLoginLabel(emailState.current_email, emailState.provider)}</strong>
        </p>
      )}

      <hr style={{ margin: '2rem 0', borderColor: '#eee' }} />

      <h3>Account Actions</h3>
      <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'center' }}>
        {emailState?.can_change_email ? (
          <button
            type="button"
            onClick={() => navigate('/change-email')}
            style={{
              padding: '0.6rem 1.2rem',
              cursor: 'pointer',
            }}
          >
            Change Email
          </button>
        ) : (
          <p style={{ color: '#666', fontSize: '0.875rem', margin: 0 }}>
            Your email is managed by your sign-in provider.
          </p>
        )}

        <button
          type="button"
          onClick={handleLogout}
          disabled={loggingOut}
          style={{
            padding: '0.6rem 1.2rem',
            background: '#d32f2f',
            color: '#fff',
            border: 'none',
            borderRadius: '4px',
            cursor: loggingOut ? 'not-allowed' : 'pointer',
          }}
        >
          {loggingOut ? 'Logging out...' : 'Log Out'}
        </button>
      </div>
    </div>
  );
}
