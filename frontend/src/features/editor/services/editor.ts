import { untrack } from "solid-js";
import { unwrap } from "solid-js/store";

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

export async function openFile(filepath: string) {
  const id = `${Panel.FilePrefix}${filepath}`;
  const filename = filepath.split(FilePath.Separator).pop();
  const api = untrack(dockview);

  // The welcome message can arrive before the lazy panel bundle has mounted.
  // Queue the request and let Panels flush it after Dockview is ready.
  if (!api) {
    pendingFiles.add(filepath);
    return;
  }

  if (!api.getPanel(id)) {
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
  });
}
