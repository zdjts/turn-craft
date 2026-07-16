import type { RoomTemplate } from '../games/types';
import { getAllGames, getGameDef } from '../games/registry';

interface Props {
  selectedGame: string | null;
  onSelect: (gt: string) => void;
  onQuickStart: (gt: string) => void;
  onEnterConfig: (gt: string) => void;
  onTemplateStart: (gt: string, tpl: RoomTemplate) => void;
}

function TierBadge({ tier }: { tier: string }) {
  if (tier === 'main') {
    return <span className="tier-badge recommend">推荐</span>;
  }
  return <span className="tier-badge experimental">实验</span>;
}

export default function GameBrowseView({
  selectedGame,
  onSelect,
  onQuickStart,
  onEnterConfig,
  onTemplateStart,
}: Props) {
  const games = getAllGames();
  const selectedDef = selectedGame ? getGameDef(selectedGame) : null;
  const templates = selectedDef?.templates ?? [];

  return (
    <>
      {selectedDef && (
        <div className="pg-lobby-quick-guide g-card">
          <div className="pg-lobby-quick-guide-title">
            📖 游戏说明
            <TierBadge tier={selectedDef.tier} />
          </div>
          {selectedDef.helpText.map((line, i) => (
            <div key={i} className="pg-lobby-quick-guide-line">{line}</div>
          ))}
          {selectedDef.tier === 'experimental' && (
            <div className="g-warning" style={{ marginTop: 12, padding: 8, borderRadius: 6, background: 'rgba(255,200,0,0.1)', color: '#c90' }}>
              ⚠️ 此游戏处于实验阶段，体验可能不稳定。
            </div>
          )}
        </div>
      )}

      {templates.length > 0 && (
        <div className="pg-lobby-templates g-card">
          <div className="pg-lobby-quick-guide-title">🎮 从模板开局</div>
          <div className="pg-lobby-template-list">
            {templates.map((tpl, idx) => (
              <div
                key={idx}
                className="pg-lobby-template-card g-card-subtle"
                onClick={() => onTemplateStart(selectedDef!.gameType, tpl)}
              >
                <span className="pg-lobby-template-icon">{tpl.icon}</span>
                <div className="pg-lobby-template-name">{tpl.name}</div>
                <div className="pg-lobby-template-desc">{tpl.desc}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="pg-lobby-games">
        <div className="pg-lobby-recommend g-card">
          <div className="pg-lobby-recommend-title">🔥 推荐玩法</div>
          <div className="pg-lobby-recommend-list">
            {games.map((def) => {
              const firstTpl = def.templates[0];
              const desc = firstTpl ? `${firstTpl.name} — ${firstTpl.desc}` : def.description;
              return (
                <div
                  key={def.gameType}
                  className="pg-lobby-recommend-card g-card-subtle"
                  onClick={() => onQuickStart(def.gameType)}
                >
                  <span className="pg-lobby-recommend-icon">{def.icon}</span>
                  <div className="pg-lobby-recommend-body">
                    <div className="pg-lobby-recommend-name">
                      {def.name}
                      <TierBadge tier={def.tier} />
                    </div>
                    <div className="pg-lobby-recommend-desc">{desc}</div>
                  </div>
                  <span className="pg-lobby-recommend-tag">{def.minPlayers}-{def.maxPlayers}人</span>
                </div>
              );
            })}
          </div>
        </div>

        {games.map((def) => {
          const isSelected = selectedGame === def.gameType;
          return (
            <div
              key={def.gameType}
              className={isSelected ? 'pg-lobby-game-card is-selected' : 'pg-lobby-game-card'}
              onClick={() => onSelect(def.gameType)}
            >
              <div className="pg-lobby-game-icon">{def.icon}</div>
              <div className="pg-lobby-game-name">
                {def.name}
                <TierBadge tier={def.tier} />
              </div>
              <div className="pg-lobby-game-card-desc">{def.description}</div>
              <div className="pg-lobby-game-card-meta">{def.minPlayers}-{def.maxPlayers} 人</div>
              <div className="pg-lobby-game-card-actions">
                <div className="pg-lobby-game-card-actions-inner">
                  <button
                    className="pg-lobby-quick-start"
                    onClick={(e) => { e.stopPropagation(); onQuickStart(def.gameType); }}
                  >
                    ⚡ 快速开始
                  </button>
                  <button
                    className="pg-lobby-config-toggle"
                    onClick={(e) => { e.stopPropagation(); onEnterConfig(def.gameType); }}
                  >
                    ⚙ 自定义
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}
