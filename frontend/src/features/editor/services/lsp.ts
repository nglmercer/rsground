import {
  LSPClient,
  LSPPlugin,
  Workspace,
  type LSPClientExtension,
  type WorkspaceFile,
  findReferences,
  formatDocument,
  hoverTooltips,
  jumpToDefinition,
  renameSymbol,
  serverCompletion,
  serverDiagnostics,
  signatureHelp,
} from "@codemirror/lsp-client";
import {
  ChangeSet,
  Extension,
  Text,
  type TransactionSpec,
} from "@codemirror/state";
import { EditorView, keymap, type Command } from "@codemirror/view";

import { openFile, waitForEditor } from "./editor";
import { onWebSocketState, onWsMessage, sendMessage } from "@features/ws/services";
import { wsSession, wsSessionId } from "@features/ws/stores";
import {
  ClientMessageKind,
  DocumentInfo,
  ServerMessageKind,
} from "@features/ws/types";

import { semanticTokensClientExtension } from "../utils/semanticTokens";
import {
  fileUri,
  pathFromUri,
  sanitizeLspHtml,
} from "../utils/lspUtils";

export { fileUri, pathFromUri, sanitizeLspHtml } from "../utils/lspUtils";
const LSP_TIMEOUT_MS = 10_000;

type LanguageServerHandler = (message: string) => void;

type ProjectFile = {
  text: string;
  revision: number;
};

class LanguageServerTransport {
  private handlers = new Set<LanguageServerHandler>();
  private active = false;

  constructor() {
    this.active = Boolean(wsSession());
  }

  setActive(active: boolean) {
    this.active = active;
  }

  send(message: string) {
    if (!this.active) throw new Error("WebSocket is not connected");

    let parsed: unknown;

    try {
      parsed = JSON.parse(message);
    } catch (error) {
      throw new Error(`Invalid LSP message: ${String(error)}`);
    }

    if (
      parsed === null ||
      typeof parsed !== "object" ||
      Array.isArray(parsed)
    ) {
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

class PlaygroundFile implements WorkspaceFile {
  constructor(
    readonly uri: string,
    readonly languageId: string,
    public version: number,
    public doc: Text,
    public view: EditorView | null,
  ) {}

  getView() {
    return this.view;
  }
}

class PlaygroundLspWorkspace extends Workspace {
  files: PlaygroundFile[] = [];

  private readonly filesByUri = new Map<string, PlaygroundFile>();
  private readonly versions = new Map<string, number>();

  setProjectFiles(files: Map<string, ProjectFile>) {
    const rustFiles = new Set(
      [...files.keys()].filter((path) => path.endsWith(".rs")),
    );

    for (const file of [...this.files]) {
      const path = pathFromUri(file.uri);
      if (path && !rustFiles.has(path)) {
        if (this.client.connected) this.client.didClose(file.uri);
        this.filesByUri.delete(file.uri);
        this.files = this.files.filter((candidate) => candidate !== file);
      }
    }

    for (const [path, projectFile] of files) {
      if (!path.endsWith(".rs")) continue;

      const uri = fileUri(path);
      const existing = this.filesByUri.get(uri);
      if (!existing) {
        const file = new PlaygroundFile(
          uri,
          "rust",
          this.nextVersion(path),
          textFromString(projectFile.text),
          null,
        );
        this.files.push(file);
        this.filesByUri.set(uri, file);
        continue;
      }

      // An active CodeMirror view owns its document synchronization. Remote
      // edits will arrive through the normal editor sync extension, so do not
      // replace its document underneath the view.
      if (!existing.view && existing.doc.toString() !== projectFile.text) {
        existing.doc = textFromString(projectFile.text);
        this.sendExternalChange(existing);
      }
    }
  }

  syncExternalFile(path: string, text: string) {
    const file = this.filesByUri.get(fileUri(path));
    if (!file || file.view) return;
    if (file.doc.toString() === text) return;

    file.doc = textFromString(text);
    this.sendExternalChange(file);
  }

  private sendExternalChange(file: PlaygroundFile) {
    if (!this.client.connected) return;

    file.version = this.nextVersion(pathFromUri(file.uri) ?? file.uri);
    this.client.notification("textDocument/didChange", {
      textDocument: {
        uri: file.uri,
        version: file.version,
      },
      contentChanges: [{ text: file.doc.toString() }],
    });
  }

  private nextVersion(path: string) {
    const version = (this.versions.get(path) ?? -1) + 1;
    this.versions.set(path, version);
    return version;
  }

  override getFile(uri: string) {
    return this.filesByUri.get(uri) ?? null;
  }

  override requestFile(uri: string) {
    return Promise.resolve(this.getFile(uri));
  }

  override syncFiles() {
    const result: Array<{
      file: PlaygroundFile;
      prevDoc: Text;
      changes: ChangeSet;
    }> = [];

    for (const file of this.files) {
      if (!file.view) continue;

      const plugin = LSPPlugin.get(file.view);
      if (!plugin || plugin.unsyncedChanges.empty) continue;

      result.push({
        file,
        prevDoc: file.doc,
        changes: plugin.unsyncedChanges,
      });
      file.doc = file.view.state.doc;
      file.version = this.nextVersion(pathFromUri(file.uri) ?? file.uri);
      plugin.clear();
    }

    return result;
  }

  override connected() {
    for (const file of this.files) {
      if (file.view) {
        const plugin = LSPPlugin.get(file.view);
        plugin?.clear();
        file.doc = file.view.state.doc;
      }

      this.client.didOpen(file);
    }
  }

  override disconnected() {
    for (const file of this.files) this.client.didClose(file.uri);
  }

  override openFile(uri: string, languageId: string, view: EditorView) {
    const path = pathFromUri(uri);
    if (!path || languageId !== "rust") return;

    let file = this.filesByUri.get(uri);
    if (!file) {
      file = new PlaygroundFile(
        uri,
        languageId,
        this.nextVersion(path),
        view.state.doc,
        view,
      );
      this.files.push(file);
      this.filesByUri.set(uri, file);
      this.client.didOpen(file);
      return;
    }

    file.view = view;
    file.doc = view.state.doc;

    // Project files are opened virtually so Rust Analyzer can resolve symbols
    // across files. Re-send the current text when a real editor is attached
    // so diagnostics and semantic tokens are published for that view as well.
    this.sendExternalChange(file);
  }

  override closeFile(uri: string, view: EditorView) {
    const file = this.filesByUri.get(uri);
    if (file?.view === view) file.view = null;
  }

  override updateFile(uri: string, update: TransactionSpec) {
    const file = this.filesByUri.get(uri);
    if (!file) return;

    const view = file.view;
    if (view) {
      view.dispatch(update);
      return;
    }

    // Workspace edits, such as a rename, must enter the normal collaborative
    // editor path. Opening a hidden file first lets the sync extension turn
    // the LSP edit into a regular project operation.
    void this.displayFile(uri).then((opened) => opened?.dispatch(update));
  }

  override async displayFile(uri: string) {
    const file = this.filesByUri.get(uri);
    const path = pathFromUri(uri);
    if (!file || !path) return null;

    const current = file.getView();
    if (current) {
      current.focus();
      return current;
    }

    await openFile(path);
    const view = await waitForEditor(path);
    view?.focus();
    return view;
  }
}

let client: LSPClient | null = null;
let transport: LanguageServerTransport | null = null;
let workspace: PlaygroundLspWorkspace | null = null;
let activeSessionId = wsSessionId();
const projectFiles = new Map<string, ProjectFile>();

const editableCommand = (command: Command): Command => (view) => {
  if (view.state.readOnly) return false;
  return command(view);
};

const clientExtensions: readonly (Extension | LSPClientExtension)[] = [
  // This is the only completion extension in a Rust editor. The app's
  // generic fallback completion is kept for non-Rust files below, avoiding
  // two CodeMirror completion state fields competing in the same editor.
  serverCompletion({ override: true }),
  hoverTooltips(),
  keymap.of([
    {
      key: "Shift-Alt-f",
      run: editableCommand(formatDocument),
      preventDefault: true,
    },
    { key: "F2", run: editableCommand(renameSymbol), preventDefault: true },
    { key: "F12", run: jumpToDefinition, preventDefault: true },
    { key: "Shift-F12", run: findReferences, preventDefault: true },
  ]),
  signatureHelp(),
  serverDiagnostics(),
  semanticTokensClientExtension(),
];

onWsMessage(ServerMessageKind.Lsp, (message) => {
  transport?.receive(message.message);
});

onWsMessage(ServerMessageKind.Welcome, (message) => {
  activeSessionId = message.session_id;
  workspace?.setProjectFiles(projectFiles);
  reconnectClient();
});

onWebSocketState((state) => {
  transport?.setActive(state === "open");
  if (state === "closed") {
    activeSessionId = null;
    client?.disconnect();
  }
});

function reconnectClient() {
  if (!client || !transport || !activeSessionId) return;

  client.disconnect();
  client.connect(transport);
}

function getClient() {
  if (client) return client;

  transport = new LanguageServerTransport();
  client = new LSPClient({
    rootUri: "file:///home",
    timeout: LSP_TIMEOUT_MS,
    workspace: (nextClient) => {
      workspace = new PlaygroundLspWorkspace(nextClient);
      workspace.setProjectFiles(projectFiles);
      return workspace;
    },
    extensions: clientExtensions,
    sanitizeHTML: sanitizeLspHtml,
  });

  if (activeSessionId) client.connect(transport);
  return client;
}

export function syncLspProjectFiles(files: Record<string, DocumentInfo>) {
  projectFiles.clear();
  for (const [path, file] of Object.entries(files)) {
    projectFiles.set(path, { text: file.text, revision: file.revision });
  }
  workspace?.setProjectFiles(projectFiles);
}

export function syncLspProjectFile(path: string, text: string) {
  const current = projectFiles.get(path);
  projectFiles.set(path, {
    text,
    revision: (current?.revision ?? -1) + 1,
  });
  workspace?.syncExternalFile(path, text);
}

export function removeLspProjectFile(path: string) {
  projectFiles.delete(path);
  workspace?.setProjectFiles(projectFiles);
}

export function rustAnalyzerExtensions(file: string): Extension[] {
  return [getClient().plugin(fileUri(file), "rust")];
}

function textFromString(value: string) {
  return Text.of(value.split("\n"));
}
