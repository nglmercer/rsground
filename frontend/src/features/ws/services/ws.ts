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

let subs: (() => void)[] = [];
export function startWebsocket() {
  subs.forEach((sub) => sub());
  subs = [];

  subs.push(
    observable(() => [authInfo(), projectInfo.id] as const).subscribe(
      ([authInfo, projectId]) => {
        if (!!authInfo?.jwt && !!projectId) {
          wsSession()?.close();
          connectWs(authInfo.jwt, projectId);
        } else {
          setWsSession(null);
        }
      },
    ).unsubscribe,
  );

  subs.push(
    observable(wsSession).subscribe((wsSession) => {
      if (wsSession) {
        for (const msg of wsQueue) {
          wsSession.send(JSON.stringify(msg));
        }
        clearWsQueue();
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

function connectWs(jwt: string, projectId: string) {
  const session = new WebSocket(
    wsUrl + projectId,
    [`${WebSocketConfig.AuthProtocolPrefix}${jwt}`],
  );

  let interval: NodeJS.Timeout;
  session.addEventListener("open", () => {
    setWsSession(session);

    interval = setInterval(() => {
      session.send(WebSocketConfig.Ping);
    }, WebSocketConfig.HeartbeatIntervalMs)
  });

  session.addEventListener("message", (ev) => {
    clearWsQueue();
    if (ev.data === WebSocketConfig.Ping) return;
    let data = JSON.parse(ev.data) as ServerMessage;
    console.debug("[WS] received:", data);

    for (const cb of ws_callbacks) {
      untrack(() => cb(data));
    }
  });

  session.addEventListener("error", (ev) => {
    clearInterval(interval);
    setWsSession(null);
    console.error("Websocket error:", ev);
  });

  session.addEventListener("close", (ev) => {
    clearInterval(interval);
    setWsSession(null);
    console.error("Websocket closed:", ev);

    if (ev.code === WebSocketConfig.AbnormalClosureCode) startWebsocket();
  });
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
  if (session) {
    clearWsQueue();
    wsQueue.push(msg_action);
    session.send(JSON.stringify(msg_action));
  } else {
    wsQueue.push(msg_action);
  }
}
