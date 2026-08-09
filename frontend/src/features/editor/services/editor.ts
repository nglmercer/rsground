import { untrack } from "solid-js";
import { unwrap } from "solid-js/store";
import { EditorView } from "codemirror";

import { dockview } from "@features/panels/stores";
import { onWsMessage } from "@features/ws/services";
import { ServerMessageKind } from "@features/ws/types";

import {
  editingFiles,
  setEditingFiles,
  setSyncFiles,
  syncFiles,
} from "../stores";
import { applyOperationToString } from "../utils";
import { OpSeq } from "frontend-wasm";
import { DockviewConfig, EditingFileField, FilePath, Panel } from "@constants";

let syncListenerStarted = false;
const pendingFiles = new Set<string>();
const editorViews = new Map<string, EditorView>();
const editorWaiters = new Map<string, Set<(view: EditorView) => void>>();

export function registerEditorView(file: string, view: EditorView) {
  editorViews.set(file, view);

  const waiters = editorWaiters.get(file);
  if (!waiters) return;

  editorWaiters.delete(file);
  for (const resolve of waiters) resolve(view);
}

export function unregisterEditorView(file: string, view: EditorView) {
  if (editorViews.get(file) === view) editorViews.delete(file);
}

export function getEditorView(file: string) {
  return editorViews.get(file) ?? null;
}

export function waitForEditor(file: string, timeoutMs = 5_000) {
  const current = getEditorView(file);
  if (current) return Promise.resolve<EditorView | null>(current);

  return new Promise<EditorView | null>((resolve) => {
    const waiters = editorWaiters.get(file) ?? new Set();
    editorWaiters.set(file, waiters);

    let timeout: ReturnType<typeof setTimeout>;
    const resolveEditor = (view: EditorView) => {
      clearTimeout(timeout);
      resolve(view);
    };

    timeout = setTimeout(() => {
      waiters.delete(resolveEditor);
      if (!waiters.size) editorWaiters.delete(file);
      resolve(null);
    }, timeoutMs);
    waiters.add(resolveEditor);
  });
}

export async function openFile(filepath: string) {
  const id = `${Panel.FilePrefix}${filepath}`;
  const filename = filepath.split(FilePath.Separator).pop();
  const api = untrack(dockview);

  // The welcome message can arrive before the lazy panel bundle has mounted.
  // Queue the request and let Panels flush it after Dockview is ready.
  if (!api) {
    pendingFiles.add(filepath);
    return waitForEditor(filepath);
  }

  const panel = api.getPanel(id);
  if (panel) {
    panel.api.setActive();
    return waitForEditor(filepath);
  }

  if (!panel) {
    api.addPanel({
      id,
      component: Panel.Code,
      title: filename,
      position: {
        direction: DockviewConfig.CodeDirection,
        referencePanel: Panel.Output,
      },
    });
  }

  return waitForEditor(filepath);
}

export function flushPendingFiles() {
  const files = [...pendingFiles];
  pendingFiles.clear();

  for (const file of files) void openFile(file);
}

export function startReceivingSync() {
  if (syncListenerStarted) return;
  syncListenerStarted = true;

  onWsMessage(ServerMessageKind.Sync, (msg) => {
    if (unwrap(editingFiles)[msg.file]?.[EditingFileField.EditorOpen]) return;

    setEditingFiles(msg.file, EditingFileField.SyncedRevision, msg.revision);

    let content = unwrap(syncFiles)[msg.file];

    if (content != null) {
      content = msg.actions.reduce(
        (content, action) =>
          applyOperationToString(
            content,
            OpSeq.from_str(JSON.stringify(action.operation)),
          ),
        content,
      );
    }

    setSyncFiles(msg.file, content ?? "");
    void import("./lsp").then(({ syncLspProjectFile }) => {
      syncLspProjectFile(msg.file, content ?? "");
    });
  });
}
