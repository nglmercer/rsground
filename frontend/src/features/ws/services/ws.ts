import { getOwner, observable, Owner, runWithOwner, untrack } from "solid-js";

import { authInfo } from "@features/auth/stores";
import { projectInfo, setProjectInfo } from "@features/colab/stores";
import { syncFiles } from "@features/file-explorer/services";
import { showToast } from "@services/toast";
import { BACKEND_HOST } from "@services";
import {
  ProjectDefaults,
  ProjectInfoField,
  ToastKind,
  WebSocketConfig,
} from "@constants";

import {
  clearWsQueue,
  setWsSession,
  setWsSessionId,
  wsQueue,
  wsSession,
} from "../stores";
import {
  ClientMessage,
  ClientMessageKind,
  ServerMessage,
  ServerMessageKind,
  WsCallback,
} from "../types";

let wsOwner: Owner;
let activeSocket: WebSocket | null = null;
const expectedCloseSessions = new WeakSet<WebSocket>();

let subs: (() => void)[] = [];
export function startWebsocket() {
  closeActiveSocket();

  subs.forEach((sub) => sub());
  subs = [];

  subs.push(
    observable(() => [authInfo(), projectInfo.id] as const).subscribe(
      ([authInfo, projectId]) => {
        if (!!authInfo?.jwt && !!projectId) {
          closeActiveSocket();
          connectWs(authInfo.jwt, projectId);
        } else {
          closeActiveSocket();
          setWsSession(null);
          setWsSessionId(null);
        }
      },
    ).unsubscribe,
  );

  subs.push(
    observable(wsSession).subscribe((wsSession) => {
      if (wsSession) {
        flushWsQueue(wsSession);
      }
    }).unsubscribe,
  );

  wsOwner = getOwner() ?? wsOwner;

  subs.push(onWsMessage(ServerMessageKind.Welcome, (msg) => {
    setWsSessionId(msg.session_id);

    if (msg.requests) setProjectInfo(ProjectInfoField.Requests, msg.requests);

    syncFiles(msg.files);

    void import("@features/editor/services").then(({ openFile }) => {
      openFile(ProjectDefaults.MainFile);
    });
  }));

  subs.push(onWsMessage(ServerMessageKind.Error, (msg) => {
    void showToast(ToastKind.Error, {
      titleText: "Workspace action failed",
      text: msg.message,
    });
  }));
}

const wsUrl = new URL(BACKEND_HOST);
wsUrl.protocol = wsUrl.protocol === "http:" ? "ws:" : "wss:";
wsUrl.pathname = WebSocketConfig.Path;

const ws_callbacks: Array<(msg: ServerMessage) => void> = [];
const ws_state_callbacks: Array<(state: WebSocketState) => void> = [];

export type WebSocketState = "open" | "closed";

function notifyWebSocketState(state: WebSocketState) {
  for (const cb of ws_state_callbacks) cb(state);
}

function closeActiveSocket() {
  const session = activeSocket;
  if (!session) return;

  expectedCloseSessions.add(session);
  activeSocket = null;

  if (wsSession() === session) {
    setWsSession(null);
    setWsSessionId(null);
    notifyWebSocketState("closed");
  }

  if (
    session.readyState === WebSocket.CONNECTING ||
    session.readyState === WebSocket.OPEN
  ) {
    session.close(1000, "replaced");
  }
}

function flushWsQueue(session: WebSocket) {
  if (session.readyState !== WebSocket.OPEN || wsQueue.length === 0) return;

  const pending = wsQueue.slice();
  clearWsQueue();

  for (let index = 0; index < pending.length; index++) {
    if (session.readyState !== WebSocket.OPEN) {
      wsQueue.unshift(...pending.slice(index));
      return;
    }

    try {
      session.send(JSON.stringify(pending[index]));
    } catch {
      wsQueue.unshift(...pending.slice(index));
      return;
    }
  }
}

function connectWs(jwt: string, projectId: string) {
  const session = new WebSocket(
    wsUrl + projectId,
    [`${WebSocketConfig.AuthProtocolPrefix}${jwt}`],
  );
  activeSocket = session;

  let interval: ReturnType<typeof setInterval> | undefined;
  let closedNotified = false;

  const markClosed = () => {
    const expected = expectedCloseSessions.has(session);
    const current = activeSocket === session;

    if (closedNotified) {
      return { expected, current: false, first: false };
    }

    closedNotified = true;
    if (interval) clearInterval(interval);

    // A stale socket must not tear down a newer project connection.
    if (current) activeSocket = null;

    if (wsSession() === session) {
      setWsSession(null);
      setWsSessionId(null);
      notifyWebSocketState("closed");
    }

    return { expected, current, first: true };
  };

  session.addEventListener("open", () => {
    if (
      activeSocket !== session ||
      expectedCloseSessions.has(session)
    ) {
      expectedCloseSessions.add(session);
      session.close(1000, "replaced");
      return;
    }

    setWsSession(session);
    notifyWebSocketState("open");

    interval = setInterval(() => {
      if (session.readyState === WebSocket.OPEN) {
        session.send(WebSocketConfig.Ping);
      }
    }, WebSocketConfig.HeartbeatIntervalMs);
  });

  session.addEventListener("message", (ev) => {
    if (ev.data === WebSocketConfig.Ping) return;
    let data = JSON.parse(ev.data) as ServerMessage;
    console.debug("[WS] received:", data);

    for (const cb of ws_callbacks) {
      untrack(() => cb(data));
    }
  });

  session.addEventListener("error", (ev) => {
    if (!expectedCloseSessions.has(session) && activeSocket === session) {
      console.warn("[WS] WebSocket transport error; waiting for close", ev);
    }
  });

  session.addEventListener("close", (ev) => {
    const state = markClosed();
    if (!state.first) return;

    const shouldReconnect =
      state.current &&
      !state.expected &&
      (ev.code === WebSocketConfig.GoingAwayCode ||
        ev.code === WebSocketConfig.AbnormalClosureCode ||
        ev.code === WebSocketConfig.ServiceRestartCode ||
        ev.code === WebSocketConfig.TryAgainLaterCode);

    if (shouldReconnect) {
      if (ev.code !== WebSocketConfig.GoingAwayCode) {
        console.warn("[WS] reconnecting after websocket close", {
          code: ev.code,
          reason: ev.reason,
        });
      }
      startWebsocket();
    }
  });
}

export function onWebSocketState(cb: (state: WebSocketState) => void) {
  ws_state_callbacks.push(cb);

  return () => {
    const index = ws_state_callbacks.indexOf(cb);
    if (index >= 0) ws_state_callbacks.splice(index, 1);
  };
}

/** Register callback for websocket messages. Returns unsubscribe */
export function onWsMessage(cb: WsCallback): () => void;

/** Register callback for specific websocket messages. Returns unsubscribe */
export function onWsMessage<A extends ServerMessageKind>(
  action: A,
  cb: WsCallback<A>,
): () => void;
/** Register callback for specific websocket messages. Returns unsubscribe */
export function onWsMessage<A extends ServerMessageKind>(
  actions: A[],
  cb: WsCallback<A>,
): () => void;

export function onWsMessage(
  actions_or_cb: string | string[] | WsCallback,
  maybe_cb?: WsCallback,
): () => void {
  let owner = getOwner();
  let cb: WsCallback;

  if (actions_or_cb instanceof Array) {
    cb = (msg) => {
      if (actions_or_cb.includes(msg.action)) {
        runWithOwner(owner ?? wsOwner, () => maybe_cb!(msg));
      }
    };
  } else if (typeof actions_or_cb === "string") {
    cb = (msg) => {
      if (actions_or_cb === msg.action) {
        runWithOwner(owner ?? wsOwner, () => maybe_cb!(msg));
      }
    };
  } else {
    cb = (msg) => {
      runWithOwner(owner ?? wsOwner, () => actions_or_cb(msg));
    };
  }

  ws_callbacks.push(cb);

  return () => {
    let idx = ws_callbacks.findIndex((v) => v == cb);
    if (idx >= 0) ws_callbacks.splice(idx, 1);
  };
}

export function sendMessage<A extends ClientMessageKind>(
  action: A,
  msg: Omit<ClientMessage<A>, "action">,
) {
  const msg_action = { action, ...msg } as ClientMessage<A>;
  console.log("[WS] Sending message:", msg_action);

  const session = untrack(wsSession);
  if (session?.readyState === WebSocket.OPEN) {
    flushWsQueue(session);

    if (session.readyState !== WebSocket.OPEN) {
      wsQueue.push(msg_action);
      return;
    }

    try {
      session.send(JSON.stringify(msg_action));
    } catch {
      wsQueue.push(msg_action);
    }
  } else {
    wsQueue.push(msg_action);
  }
}
