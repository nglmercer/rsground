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

export async function openFile(filepath: string) {
  const id = `${Panel.FilePrefix}${filepath}`;
  const filename = filepath.split(FilePath.Separator).pop();

  if (!untrack(dockview).getPanel(id)) {
    untrack(dockview).addPanel({
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
