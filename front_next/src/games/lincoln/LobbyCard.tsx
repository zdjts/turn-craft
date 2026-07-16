import { useEffect } from 'react';
import type { GameConfigProps } from '../types';

const ROLES: [string, string][] = [
  ['Judge', '裁判 — 开题与总结'],
  ['Pro', '正方 — 立论'],
  ['Con', '反方 — 驳论'],
];

export default function LincolnLobbyCard(props: GameConfigProps) {
  const { roleConfig, myRole, maxRound, onChange } = props;

  useEffect(() => {
    if (!myRole || !['Judge', 'Pro', 'Con'].includes(myRole)) {
      onChange({
        myRole: 'Judge',
        roleConfig: { Judge: 'human', Pro: 'ai', Con: 'ai' },
        gameConfig: null,
      });
    }
  }, []);

  const selectRole = (role: string) => {
    const cfg = { ...roleConfig };
    cfg[role] = 'human';
    onChange({ myRole: role, roleConfig: cfg });
  };

  const toggleMode = (role: string) => {
    const cfg = { ...roleConfig };
    const current = cfg[role] ?? 'ai';
    if (current === 'human') {
      if (myRole !== role) cfg[role] = 'ai';
    } else {
      cfg[role] = 'human';
    }
    onChange({ roleConfig: cfg });
  };

  return (
    <>
      <div className="g-field">
        <label>角色配置</label>
        <div className="role-grid">
          {ROLES.map(([roleName, roleDesc]) => {
            const isSelected = myRole === roleName;
            const mode = roleConfig[roleName] ?? 'ai';
            const isHuman = mode === 'human';
            return (
              <div key={roleName} className={isSelected ? 'role-card selected' : 'role-card'}>
                <div className="role-card-header">
                  <span
                    className="role-card-name"
                    style={{ cursor: 'pointer' }}
                    onClick={() => selectRole(roleName)}
                  >
                    {isSelected ? '👉 ' : ''}{roleName}（我的角色）
                  </span>
                  <button
                    className={isHuman ? 'g-badge-success' : 'g-badge-info'}
                    style={{ border: 'none', cursor: 'pointer' }}
                    onClick={(e) => { e.stopPropagation(); toggleMode(roleName); }}
                  >
                    {isHuman ? '开放联机' : 'AI 接管'}
                  </button>
                </div>
                <div className="role-card-desc">{roleDesc}</div>
              </div>
            );
          })}
        </div>
      </div>
      <div className="g-field">
        <label>最大轮次</label>
        <input
          type="number"
          placeholder="16"
          value={maxRound}
          onChange={(e) => onChange({ maxRound: parseInt(e.target.value) || 0 })}
        />
      </div>
    </>
  );
}
