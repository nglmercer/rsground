import { LSPClient, languageServerExtensions } from "@codemirror/lsp-client";
import type { Extension } from "@codemirror/state";

import { onWsMessage, sendMessage } from "@features/ws/services";
import {
  ClientMessageKind,
  ServerMessageKind,
} from "@features/ws/types";

let client: LSPClient | null = null;
let transport: LanguageServerTransport | null = null;

type LanguageServerHandler = (message: string) => void;

class LanguageServerTransport {
  private handlers = new Set<LanguageServerHandler>();

  send(message: string) {
    let parsed: unknown;

    try {
      parsed = JSON.parse(message);
    } catch (error) {
      throw new Error(`Invalid LSP message: ${String(error)}`);
    }

    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("LSP messages must be JSON objects");
    }

    sendMessage(ClientMessageKind.Lsp, {
      message: parsed as Record<string, unknown>,
    });
  }

  subscribe(handler: LanguageServerHandler) {
    this.handlers.add(handler);
  }

  unsubscribe(handler: LanguageServerHandler) {
    this.handlers.delete(handler);
  }

  receive(message: Record<string, unknown>) {
    const serialized = JSON.stringify(message);
    for (const handler of this.handlers) handler(serialized);
  }
}

onWsMessage(ServerMessageKind.Lsp, (message) => {
  transport?.receive(message.message);
});

// A reconnect creates a fresh sandbox process. Re-run the LSP initialization
// exchange on the existing CodeMirror client so open editors are sent again.
onWsMessage(ServerMessageKind.Welcome, () => {
  if (client && transport) client.connect(transport);
});

function getClient() {
  if (client) return client;

  transport = new LanguageServerTransport();
  client = new LSPClient({
    rootUri: "file:///home",
    timeout: 10_000,
    extensions: languageServerExtensions(),
  });

  client.connect(transport);
  return client;
}

function fileUri(file: string) {
  const path = file
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");

  return `file:///home/${path}`;
}

export function rustAnalyzerExtensions(file: string): Extension[] {
  return [getClient().plugin(fileUri(file), "rust")];
}
