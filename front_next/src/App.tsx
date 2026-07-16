import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider, useAuth } from './store/AuthContext';
import AppLayout from './routes/Layout';
import Login from './routes/Login';
import Lobby from './routes/Lobby';
import History from './routes/History';
import PublicRooms from './routes/Public';
import Profile from './routes/Profile';
import About from './routes/About';
import Game from './routes/Game';
import Settings from './routes/Settings';
import Replay from './routes/Replay';
import InvitePage from './routes/Invite';
import Leaderboard from './routes/Leaderboard';
import type { ReactNode } from 'react';

function RequireAuth({ children }: { children: ReactNode }) {
  const { isAuthenticated } = useAuth();
  if (!isAuthenticated) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<Login />} />
          <Route path="/invite/:code" element={<InvitePage />} />
          <Route
            element={
              <RequireAuth>
                <AppLayout />
              </RequireAuth>
            }
          >
            <Route path="/" element={<Lobby />} />
            <Route path="/history" element={<History />} />
            <Route path="/public" element={<PublicRooms />} />
            <Route path="/profile" element={<Profile />} />
            <Route path="/about" element={<About />} />
            <Route path="/leaderboard" element={<Leaderboard />} />
            <Route path="/game/:roomId/:actorId" element={<Game />} />
            <Route path="/settings/:roomId/:actorId" element={<Settings />} />
            <Route path="/replay/:roomId" element={<Replay />} />
          </Route>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}
