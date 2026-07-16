import { useState, useEffect, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { createRoom, getPublicRooms, joinRoom } from '../api/rooms';
import type { RoomSnapshot } from '../api/rooms';
import { getGameDef } from '../games/registry';
import type { GameConfigProps, RoomTemplate } from '../games/types';
import GameBrowseView from '../components/GameBrowseView';
import GameConfigView from '../components/GameConfigView';
import PublicRoomList from '../components/PublicRoomList';

type Mode = { kind: 'browse' } | { kind: 'config'; gameType: string };

function defaultConfig(gt: string): GameConfigProps {
  const def = getGameDef(gt);
  const cfg = def?.defaultConfig();
  return {
    roleConfig: cfg?.roleConfig ?? {},
    myRole: cfg?.myRole ?? '',
    maxRound: cfg?.maxRound ?? 16,
    gameConfig: cfg?.gameConfig ?? null,
    isPublic: true,
    onChange: () => {},
  } satisfies GameConfigProps;
}

export default function Lobby() {
  const navigate = useNavigate();
  const [mode, setMode] = useState<Mode>({ kind: 'browse' });
  const [selectedGame, setSelectedGame] = useState<string | null>(null);
  const [config, setConfig] = useState<GameConfigProps>(() => defaultConfig('lincoln'));
  const [creating, setCreating] = useState(false);
  const [rooms, setRooms] = useState<RoomSnapshot[]>([]);
  const [loadingRooms, setLoadingRooms] = useState(true);

  const loadRooms = useCallback(async () => {
    setLoadingRooms(true);
    try {
      const res = await getPublicRooms();
      setRooms(res.rooms ?? []);
    } catch {
      // silently fail
    } finally {
      setLoadingRooms(false);
    }
  }, []);

  useEffect(() => {
    loadRooms();
  }, [loadRooms]);

  const selectGame = (gt: string) => {
    const def = getGameDef(gt);
    const cfg = def?.defaultConfig();
    setConfig({
      roleConfig: cfg?.roleConfig ?? {},
      myRole: cfg?.myRole ?? '',
      maxRound: cfg?.maxRound ?? 16,
      gameConfig: cfg?.gameConfig ?? null,
      isPublic: config.isPublic,
      onChange: (patch) => setConfig((prev) => ({ ...prev, ...patch })),
    });
  };

  const handleCreateRoom = async (gameTypeOverride?: string, cfgOverride?: GameConfigProps) => {
    if (creating) return;
    setCreating(true);

    const cfg = cfgOverride ?? config;
    const gameType = gameTypeOverride ?? (mode.kind === 'config' ? mode.gameType : 'lincoln');
    const def = getGameDef(gameType);
    const slots = def ? def.generateSlots(cfg.roleConfig) : Object.keys(cfg.roleConfig).sort();

    try {
      const res = await createRoom({
        game_type: gameType,
        max_round: cfg.maxRound,
        my_slot: cfg.myRole,
        slots,
        slot_configs: cfg.roleConfig,
        game_config: cfg.gameConfig ?? undefined,
        is_public: cfg.isPublic,
      });

      if (res.status === 'success' && res.room_id && res.actor_id) {
        navigate(`/game/${res.room_id}/${res.actor_id}`);
      } else {
        console.error('创建房间失败', res.message);
      }
    } catch (err) {
      console.error('创建房间失败', err);
    } finally {
      setCreating(false);
    }
  };

  const quickCreate = (gt: string) => {
    const def = getGameDef(gt);
    const dcfg = def?.defaultConfig();
    const cfg: GameConfigProps = {
      roleConfig: dcfg?.roleConfig ?? {},
      myRole: dcfg?.myRole ?? '',
      maxRound: dcfg?.maxRound ?? 16,
      gameConfig: dcfg?.gameConfig ?? null,
      isPublic: true,
      onChange: () => {},
    };
    selectGame(gt);
    handleCreateRoom(gt, cfg);
  };

  const enterConfig = (gt: string) => {
    selectGame(gt);
    setSelectedGame(gt);
    setMode({ kind: 'config', gameType: gt });
  };

  const backToBrowse = () => {
    setMode({ kind: 'browse' });
  };

  const toggleSelect = (gt: string) => {
    setSelectedGame((prev) => (prev === gt ? null : gt));
  };

  const handleTemplateStart = async (gt: string, tpl: RoomTemplate) => {
    if (creating) return;
    setCreating(true);

    const def = getGameDef(gt);
    const slots = def ? def.generateSlots(tpl.roleConfig) : Object.keys(tpl.roleConfig).sort();

    try {
      const res = await createRoom({
        game_type: gt,
        max_round: tpl.maxRound,
        my_slot: tpl.myRole,
        slots,
        slot_configs: tpl.roleConfig,
        game_config: tpl.gameConfig ?? undefined,
        is_public: true,
      });

      if (res.status === 'success' && res.room_id && res.actor_id) {
        navigate(`/game/${res.room_id}/${res.actor_id}`);
      }
    } catch (err) {
      console.error('模板创建失败', err);
    } finally {
      setCreating(false);
    }
  };

  const roomFilter = mode.kind === 'config'
    ? mode.gameType
    : selectedGame ?? undefined;

  const activeRooms = useMemo(() =>
    rooms.filter((r) => {
      const engine = r.engine_state as Record<string, unknown>;
      const finished = (engine?.finished as boolean) ?? false;
      const phase = (engine?.phase as string) ?? '';
      return !finished && phase !== 'WaitingForPlayers' && phase !== '';
    }),
  [rooms]);

  const handleJoinActive = async (room: RoomSnapshot) => {
    const slots = room.actor_slots as { slot_name: string; occupant: string }[] | null;
    const emptySlot = slots?.find((s) => s.occupant === 'Empty');
    if (emptySlot) {
      try {
        await joinRoom(room.room_id, emptySlot.slot_name);
        navigate(`/game/${room.room_id}/${emptySlot.slot_name}`);
      } catch { /* ignore */ }
    } else {
      navigate(`/game/${room.room_id}/spectator`);
    }
  };

  return (
    <div className="pg-lobby animate-fade-in">
      <div className="pg-lobby-banner">
        <h1>⚔️ 欢迎来到 Turn Craft</h1>
        <p>选择您喜爱的回合制博弈游戏，搭配个性化 AI 助手，开启精彩协作！</p>
      </div>

      {activeRooms.length > 0 && (
        <div className="pg-lobby-active g-card" style={{ marginBottom: 16 }}>
          <div className="pg-lobby-rooms-header">
            <h3>🎮 活跃对局</h3>
            <button className="pg-lobby-refresh" onClick={loadRooms} title="刷新">🔄</button>
          </div>
          <div className="pg-lobby-active-list" style={{ display: 'flex', gap: 12, overflow: 'auto', padding: '8px 0' }}>
            {activeRooms.map((room) => {
              const def = getGameDef(room.game_type);
              const engine = room.engine_state as Record<string, unknown>;
              const rawPhase = engine?.phase;
              const phase = typeof rawPhase === 'object' && rawPhase
                ? Object.keys(rawPhase as Record<string, unknown>)[0] ?? ''
                : (rawPhase as string) ?? '';
              const round = (engine?.round as number) ?? 0;
              const slots = room.actor_slots as { slot_name: string; occupant: string }[] | null;
              const emptyCount = slots?.filter((s) => s.occupant === 'Empty').length ?? 0;
              return (
                <div key={room.room_id} className="g-card-subtle" style={{ minWidth: 200, padding: 12, cursor: 'pointer' }}
                  onClick={() => handleJoinActive(room)}
                >
                  <div style={{ fontSize: '1.2em' }}>{def?.icon ?? '❓'} {def?.name ?? '?'}</div>
                  <div style={{ fontSize: '0.85em', color: 'var(--text-muted)', marginTop: 4 }}>
                    {phase} · 第{round}轮 · {emptyCount > 0 ? `${emptyCount}空位` : '可观战'}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      <div className="pg-lobby-layout">
        <div className="pg-lobby-left">
          {mode.kind === 'browse' ? (
            <GameBrowseView
              selectedGame={selectedGame}
              onSelect={toggleSelect}
              onQuickStart={quickCreate}
              onEnterConfig={enterConfig}
              onTemplateStart={handleTemplateStart}
            />
          ) : (
            (() => {
              const def = getGameDef(mode.gameType);
              if (!def) return <div>未知游戏</div>;
              return (
                <GameConfigView
                  gameDef={def}
                  config={config}
                  creating={creating}
                  onCreate={handleCreateRoom}
                  onBack={backToBrowse}
                />
              );
            })()
          )}
        </div>

        <PublicRoomList
          rooms={rooms}
          loading={loadingRooms}
          roomFilter={roomFilter}
          onRefresh={loadRooms}
        />
      </div>
    </div>
  );
}
