// WebSocket client: reconnect with backoff, dispatch events into the store.

type Handler = (data: unknown) => void;

const handlers = new Map<string, Set<Handler>>();

export function onWsEvent(event: string, fn: Handler): () => void {
  if (!handlers.has(event)) handlers.set(event, new Set());
  handlers.get(event)!.add(fn);
  return () => handlers.get(event)?.delete(fn);
}

function dispatch(event: string, data: unknown) {
  handlers.get(event)?.forEach((fn) => {
    try {
      fn(data);
    } catch (e) {
      console.error("ws handler error", e);
    }
  });
}

let socket: WebSocket | null = null;
let closedByUser = false;

export function connectWs() {
  closedByUser = false;
  open();
}

export function disconnectWs() {
  closedByUser = true;
  socket?.close();
}

function open() {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  socket = new WebSocket(`${proto}://${location.host}/ws`);
  socket.onopen = () => {
    socket?.send(JSON.stringify({ type: "subscribe", topics: ["*"] }));
  };
  socket.onmessage = (ev) => {
    try {
      const msg = JSON.parse(ev.data as string);
      if (msg.type === "event") {
        dispatch(msg.event, msg.data);
      } else if (msg.type === "ready") {
        dispatch("ready", msg.session);
      }
    } catch {
      /* ignore malformed */
    }
  };
  socket.onclose = () => {
    dispatch("ws_closed", null);
    if (!closedByUser) {
      setTimeout(open, 1500);
    }
  };
  socket.onerror = () => {
    socket?.close();
  };
}
