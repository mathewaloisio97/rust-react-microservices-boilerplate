//! Root application router configuring contexts and public/private routes.

import { GoogleOAuthProvider } from '@react-oauth/google';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { GOOGLE_CLIENT_ID } from './config.js';
import Login from './pages/Login.js';
import Register from './pages/Register.js';

/**
 * Root component that initializes global providers and defines the application's routing hierarchy.
 */
export default function App() {
  return (
    <GoogleOAuthProvider clientId={GOOGLE_CLIENT_ID}>
      <BrowserRouter>
        <Routes>
          <Route path="/register" element={<Register />} />
          <Route path="/login" element={<Login />} />

          {/* Dashboard is a placeholder until we build CP-7 (Dashboard & Access Control) */}
          <Route
            path="/dashboard"
            element={
              <div style={{ padding: '20px', fontFamily: 'sans-serif' }}>
                <h1>Dashboard Placeholder</h1>
                <p>You have successfully logged in and acquired a JWT!</p>
              </div>
            }
          />

          {/* Fallback route */}
          <Route path="*" element={<Navigate to="/login" replace />} />
        </Routes>
      </BrowserRouter>
    </GoogleOAuthProvider>
  );
}
