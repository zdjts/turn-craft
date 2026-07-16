import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles/main.css';
import './styles/login.css';
import './styles/lobby.css';
import './styles/game.css';
import './styles/settings.css';
import './styles/replay.css';
import './styles/history.css';
import './styles/profile.css';
import './styles/about.css';
import './styles/public.css';
import './styles/poker.css';
import './styles/landing.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
