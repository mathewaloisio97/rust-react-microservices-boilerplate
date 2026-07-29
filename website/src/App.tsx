/**
 * Main application routing module and client entry-point shell.
 *
 * Configures client-side routing using `react-router-dom` to map URL paths
 * to authentication pages, verification flows, user settings, and application states.
 * Wraps the route tree in `GoogleOAuthProvider` to enable Google SSO across views.
 *
 * @module App
 */

import { GoogleOAuthProvider } from '@react-oauth/google';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { GOOGLE_CLIENT_ID } from './config.js';
import { ChangeEmail } from './pages/ChangeEmail.js';
import { Dashboard } from './pages/Dashboard.js';
import { Login } from './pages/Login.js';
import { Register } from './pages/Register.js';
import { VerifyEmail } from './pages/VerifyEmail.js';

/**
 * Root Route Evaluator Component.
 *
 * Inspects local storage for an active session token:
 * - If authenticated, redirects to `/dashboard`.
 * - If unauthenticated, redirects to `/login`.
 */
function RootRedirect() {
  const hasSession = Boolean(localStorage.getItem('sessionToken'));
  return <Navigate to={hasSession ? '/dashboard' : '/login'} replace />;
}

/**
 * Restricted View: Account suspension notice page.
 *
 * Displayed when an authenticated session detects that the underlying account
 * status has been set to SUSPENDED by administrative or security controls.
 */
function Suspended() {
  return (
    <div style={{ padding: '2rem', color: 'red', fontFamily: 'sans-serif' }}>
      <h1>Account Suspended</h1>
      <p>Your access has been temporarily or permanently restricted.</p>
    </div>
  );
}

/**
 * Root Application Component.
 *
 * Wraps the top-level route tree in an HTML5 history API router (`BrowserRouter`)
 * and a `GoogleOAuthProvider` for provider authentication.
 * Configures page navigation endpoints across authentication, verification, and dashboard views:
 * - `/` -> Evaluates session status (`RootRedirect`)
 * - `/login` -> Account authentication view (`Login`)
 * - `/register` -> New account registration view (`Register`)
 * - `/verify` -> Email verification challenge view (`VerifyEmail`)
 * - `/change-email` -> Email configuration management view (`ChangeEmail`)
 * - `/dashboard` -> Main active application workspace (`Dashboard`)
 * - `/suspended` -> Account restriction notification view (`Suspended`)
 */
export function App() {
  return (
    <GoogleOAuthProvider clientId={GOOGLE_CLIENT_ID}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<RootRedirect />} />
          <Route path="/login" element={<Login />} />
          <Route path="/register" element={<Register />} />
          <Route path="/verify" element={<VerifyEmail />} />
          <Route path="/change-email" element={<ChangeEmail />} />
          <Route path="/dashboard" element={<Dashboard />} />
          <Route path="/suspended" element={<Suspended />} />
        </Routes>
      </BrowserRouter>
    </GoogleOAuthProvider>
  );
}
