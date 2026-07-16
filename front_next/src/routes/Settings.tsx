import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { getAiConfigs, updateAiConfig } from '../api/aiConfig';

export default function Settings() {
  const { roomId, actorId } = useParams<{ roomId: string; actorId: string }>();
  const navigate = useNavigate();

  const [apiKey, setApiKey] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [model, setModel] = useState('');
  const [maxTokens, setMaxTokens] = useState(2048);
  const [prompt, setPrompt] = useState('');
  const [style, setStyle] = useState('default');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [allActors, setAllActors] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  useEffect(() => {
    if (!roomId) return;
    setLoading(true);
    getAiConfigs(roomId)
      .then((res) => {
        const actors = Object.keys(res.configs).sort();
        setAllActors(actors);
        const cfg = res.configs[actorId ?? ''];
        if (cfg) {
          setApiKey(cfg.api_key);
          setBaseUrl(cfg.base_url);
          setModel(cfg.model);
          setMaxTokens(cfg.max_tokens);
          setPrompt(cfg.prompt);
          setStyle(cfg.style);
        }
      })
      .catch((err) => setError(String(err)))
      .finally(() => setLoading(false));
  }, [roomId, actorId]);

  const handleSave = async () => {
    if (!roomId || !actorId || saving) return;
    setSaving(true);
    setError('');
    try {
      await updateAiConfig(roomId, actorId, {
        api_key: apiKey,
        base_url: baseUrl,
        model,
        max_tokens: maxTokens,
        prompt,
        style,
      });
      setSuccess('AI 参数已成功更新，设置将在下一回合生效。');
      setTimeout(() => navigate(-1), 1200);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="pg-settings animate-fade-in">
      <div className="page-header">
        <h1>✨ 配置 AI 智能参数</h1>
        <p>为当前房间的 AI 助手配置专属的大语言模型接入端点和个性化 Prompt 提示词。</p>
      </div>

      {loading ? (
        <div className="loading-canvas g-card">
          <span className="g-spinner" />
          <p>正在读取 AI 配置项，请稍候...</p>
        </div>
      ) : (
        <div className="pg-settings-layout g-card">
          {allActors.length > 1 && (
            <div className="pg-settings-tabs">
              {allActors.map((a) => (
                <button
                  key={a}
                  className={a === actorId ? 'pg-settings-tab is-active' : 'pg-settings-tab'}
                  onClick={() => navigate(`/settings/${roomId}/${a}`)}
                >
                  AI {a}
                </button>
              ))}
            </div>
          )}

          <div className="pg-settings-title">
            <span className="ai-model-badge">🤖</span> 正在配置: {actorId}
          </div>

          {error && <div className="g-error">{error}</div>}
          {success && <div className="g-success">{success}</div>}

          <div className="pg-settings-form">
            <div className="pg-settings-grid">
              <div className="g-field">
                <label>API 基址 (Base URL)</label>
                <input type="text" placeholder="http://localhost:4000/v1" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
              </div>
              <div className="g-field">
                <label>模型名称 (Model)</label>
                <input type="text" placeholder="gpt-4o" value={model} onChange={(e) => setModel(e.target.value)} />
              </div>
            </div>

            <div className="pg-settings-grid">
              <div className="g-field">
                <label>API 密钥 (API Key)</label>
                <input type="password" placeholder="sk-..." value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
              </div>
              <div className="g-field">
                <label>最大输出限制 (Max Tokens)</label>
                <input type="number" placeholder="2048" value={maxTokens} onChange={(e) => setMaxTokens(parseInt(e.target.value) || 0)} />
              </div>
            </div>

            <div className="g-field">
              <label>AI 行为风格</label>
              <select className="g-select" value={style} onChange={(e) => setStyle(e.target.value)}>
                <option value="default">默认 — 无特殊风格</option>
                <option value="aggressive">激进 — 高风险高回报策略</option>
                <option value="conservative">保守 — 安全低风险策略</option>
                <option value="creative">创意 — 非传统出牌策略</option>
                <option value="deceptive">狡猾 — 虚张声势、埋陷阱、隐藏意图</option>
                <option value="rational">理性 — 逻辑链、证据驱动、逐步推理</option>
                <option value="chaotic">混乱 — 不可预测、打破常规、高颠覆性</option>
              </select>
            </div>

            <div className="g-field">
              <label>系统提示词 (System Prompt)</label>
              <textarea
                className="pg-settings-prompt"
                placeholder="输入赋予该 AI 角色的设定、推理逻辑以及遵守规则..."
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
              />
            </div>

            <div className="pg-settings-actions">
              <button className="pg-settings-cancel g-card-subtle" onClick={() => navigate(-1)}>取消</button>
              <button
                className={saving ? 'pg-settings-save is-loading' : 'pg-settings-save'}
                onClick={handleSave}
                disabled={saving}
              >
                {saving ? <span className="g-spinner" /> : '保存配置'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
