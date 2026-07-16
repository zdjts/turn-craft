import { useRef, useCallback, useState, useEffect } from 'react';
import { getToken } from '../api/client';

export type ConnState =
  | { kind: 'disconnected' }
  | { kind: 'connecting' }
  | { kind: 'connected' }
  | { kind: 'reconnecting'; attempts: number }
  | { kind: 'closed' };

export function connStateLabel(s: ConnState): string {
  switch (s.kind) {
    case 'disconnected': return '未连接';
    case 'connecting': return '正在连接...';
    case 'connected': return '已连接';
    case 'reconnecting': return '重连中...';
    case 'closed': return '房间已关闭';
  }
}

export interface WsMessage {
  type: string;
  [key: string]: unknown;
}

export interface UseWebSocketReturn {
  state: ConnState;
  lastState: unknown;
  streamingText: Record<string, string>;
  actionError: string | null;
  canRetry: boolean;
  send: (payload: unknown) => void;
  retry: () => void;
  skip: () => void;
  disconnect: () => void;
}

const MAX_RETRY_DELAY_MS = 10_000;
const INITIAL_RETRY_DELAY_MS = 500;

export function useWebSocket(roomId: string, actorId: string): UseWebSocketReturn {
  const [state, setState] = useState<ConnState>({ kind: 'disconnected' });
  const [lastState, setLastState] = useState<unknown>(null);
  const [streamingText, setStreamingText] = useState<Record<string, string>>({});
  const [actionError, setActionError] = useState<string | null>(null);
  const [canRetry, setCanRetry] = useState(false);

  const wsRef = useRef<WebSocket | null>(null);
  const retryCountRef = useRef(0);
  const retryDelayRef = useRef(INITIAL_RETRY_DELAY_MS);
  const hasReceivedStateRef = useRef(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  const buildWsUrl = useCallback(() => {
    const origin = '';
    const wsOrigin = origin.replace('http://', 'ws://').replace('https://', 'wss://');
    const token = getToken() ?? '';
    const roleParam = actorId === 'spectator' ? '&role=spectator' : '';
    return `${wsOrigin}/ws/${roomId}/${actorId}?token=${token}${roleParam}`;
  }, [roomId, actorId]);

  const cleanup = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (wsRef.current) {
      wsRef.current.onopen = null;
      wsRef.current.onclose = null;
      wsRef.current.onmessage = null;
      wsRef.current.onerror = null;
      if (wsRef.current.readyState === WebSocket.OPEN || wsRef.current.readyState === WebSocket.CONNECTING) {
        wsRef.current.close();
      }
      wsRef.current = null;
    }
  }, []);

  const connect = useCallback(() => {
    if (!mountedRef.current) return;
    cleanup();
    setState({ kind: 'connecting' });

    const url = buildWsUrl();
    let ws: WebSocket;
    try {
      ws = new WebSocket(url);
    } catch {
      scheduleReconnect();
      return;
    }
    wsRef.current = ws;

    ws.onopen = () => {
      if (!mountedRef.current) return;
      retryCountRef.current = 0;
      retryDelayRef.current = INITIAL_RETRY_DELAY_MS;
      setState({ kind: 'connected' });
    };

    ws.onmessage = (event) => {
      if (!mountedRef.current) return;
      const text = typeof event.data === 'string' ? event.data : '';
      if (!text) return;

      // Detect state message for has_received_state tracking
      if (text.includes('"type":"state"') || text.includes('"game_type"')) {
        hasReceivedStateRef.current = true;
      }

      handleDownstream(text);
    };

    ws.onclose = () => {
      if (!mountedRef.current) return;
      if (hasReceivedStateRef.current) {
        scheduleReconnect();
      } else {
        setState({ kind: 'closed' });
      }
    };

    ws.onerror = () => {
      // onclose will fire after onerror
    };
  }, [buildWsUrl, cleanup]);

  const scheduleReconnect = useCallback(() => {
    if (!mountedRef.current) return;
    retryCountRef.current += 1;
    const delay = Math.min(retryDelayRef.current, MAX_RETRY_DELAY_MS);
    setState({ kind: 'reconnecting', attempts: retryCountRef.current });

    timerRef.current = setTimeout(() => {
      if (!mountedRef.current) return;
      retryDelayRef.current = Math.min(retryDelayRef.current * 2, MAX_RETRY_DELAY_MS);
      connect();
    }, delay);
  }, [connect]);

  const handleDownstream = useCallback((text: string) => {
    let v: WsMessage;
    try {
      v = JSON.parse(text);
    } catch {
      if (text.includes('room_closed') || text.includes('ROOM_NOT_FOUND')) {
        setState({ kind: 'closed' });
      }
      return;
    }

    const msgType = v.type ?? '';

    if (msgType === 'your_hand') {
      if (v.hand) {
        setLastState((prev: unknown) => {
          if (prev && typeof prev === 'object' && !Array.isArray(prev)) {
            return { ...(prev as Record<string, unknown>), your_hand: v.hand };
          }
          return prev;
        });
      }
      return;
    }

    if (msgType === 'stream_chunk') {
      const aid = v.actor_id as string | undefined;
      const content = v.content as string | undefined;
      if (aid && content) {
        setStreamingText((prev) => ({
          ...prev,
          [aid]: (prev[aid] ?? '') + content,
        }));
      }
      return;
    }

    if (msgType === 'stream_done') {
      return;
    }

    if (msgType === 'action_error') {
      const err = (v.error as string) ?? '操作失败';
      const retry = (v.can_retry as boolean) ?? false;
      setActionError(err);
      setCanRetry(retry);
      return;
    }

    if (msgType === 'ai_exhausted') {
      const err = (v.error as string) ?? 'AI 重试耗尽';
      setActionError(err);
      setCanRetry(true);
      return;
    }

    if (msgType === 'state') {
      setActionError(null);
      setCanRetry(false);
      setLastState(v);
      setStreamingText({});
      return;
    }

    if (msgType === '') {
      if (v.error) {
        setActionError(v.error as string);
        return;
      }
      if (v.game_type) {
        setLastState(v);
        setStreamingText({});
        return;
      }
    }
  }, []);

  const send = useCallback((payload: unknown) => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(payload));
  }, []);

  const retry = useCallback(() => {
    setActionError(null);
    setCanRetry(false);
    send({ type: 'retry' });
  }, [send]);

  const skip = useCallback(() => {
    setActionError(null);
    setCanRetry(false);
    send({ type: 'skip' });
  }, [send]);

  const disconnect = useCallback(() => {
    hasReceivedStateRef.current = false;
    cleanup();
    setState({ kind: 'disconnected' });
  }, [cleanup]);

  // Connect on mount, disconnect on unmount or room change
  useEffect(() => {
    mountedRef.current = true;
    hasReceivedStateRef.current = false;
    retryCountRef.current = 0;
    retryDelayRef.current = INITIAL_RETRY_DELAY_MS;
    connect();

    return () => {
      mountedRef.current = false;
      cleanup();
    };
  }, [roomId, actorId]);

  return {
    state,
    lastState,
    streamingText,
    actionError,
    canRetry,
    send,
    retry,
    skip,
    disconnect,
  };
}
