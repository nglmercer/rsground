import { batch } from "solid-js";
import { unwrap } from "solid-js/store";

import { projectAccess } from "@features/colab/stores";
import { setCursorsFiles, setEditingFiles, setSyncFiles } from "@features/editor/stores";
import { sendMessage } from "@features/ws/services";
import {
  AccessLevel,
  ClientMessageKind,
  DocumentInfo,
} from "@features/ws/types";
import { EditingFileField } from "@constants";

import { fileExplorer, setFileExplorer } from "../stores";
import { mergeFiles } from "../utils";

export function createNewFile(file: string) {
  if (projectAccess() !== AccessLevel.Editor) return;

  sendMessage(ClientMessageKind.FileCreate, { file });
}

export function syncFiles(files: Record<string, DocumentInfo>) {
  batch(() => {
    for (const [path, doc] of Object.entries(files)) {
      setSyncFiles(path, doc.text);
      setEditingFiles(path, {
        [EditingFileField.SyncedRevision]: doc.revision,
        [EditingFileField.EditorOpen]: false,
      });
      setCursorsFiles(path, {});
    }

    const nodes = mergeFiles(Object.keys(files), unwrap(fileExplorer).nodes);

    setFileExplorer("nodes", nodes);
  });

  // Load the LSP bridge lazily so the WebSocket service and file explorer do
  // not form a module-initialization cycle during application startup.
  void import("@features/editor/services/lsp").then(({ syncLspProjectFiles }) => {
    syncLspProjectFiles(files);
  });
}
