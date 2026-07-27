//! Application entry point for the React frontend.

import { createRoot } from 'react-dom/client';
import App from './App.js';

const container = document.getElementById('root');

if (container) {
  const root = createRoot(container);
  root.render(<App />);
} else {
  console.error('FATAL: Failed to find the root DOM element.');
}
