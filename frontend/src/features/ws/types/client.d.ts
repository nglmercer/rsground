import { OpSeq } from "frontend-wasm";
import { RsCursor } from "@features/editor/types";
import { AccessLevel } from "./access";

export enum ClientMessageKind {
  Config = "config",
  PermitAccess = "permit_access",
  Execute = "execute",
  FileCreate = "file_create",
  FileDelete = "file_delete",
  StopExecute = "stop_execute",
  Sync = "sync",
  SyncCursor = "sync_cursor",
  SyncFiles = "sync_files",
  Lsp = "lsp",
}

export type ClientMessage<S extends ClientMessageKind = ClientMessageKind> = {
  [ClientMessageKind.Config]: {
    action: ClientMessageKind.Config;
    name?: string;
    is_public?: boolean;
    password?: string;
  };
  [ClientMessageKind.PermitAccess]: {
    action: ClientMessageKind.PermitAccess;
    user_id: string;
    access: AccessLevel;
  };
  [ClientMessageKind.Execute]: {
    action: ClientMessageKind.Execute;
  };
  [ClientMessageKind.FileCreate]: {
    action: ClientMessageKind.FileCreate;
    file: string;
  };
  [ClientMessageKind.FileDelete]: {
    action: ClientMessageKind.FileDelete;
    file: string;
  };
  [ClientMessageKind.StopExecute]: {
    action: ClientMessageKind.StopExecute;
  };
  [ClientMessageKind.Sync]: {
    action: ClientMessageKind.Sync;
    file: string;
    revision: number;
    actions: OpSeq;
  };
  [ClientMessageKind.SyncCursor]: {
    action: ClientMessageKind.SyncCursor;
    file: string;
    cursors: Array<RsCursor>;
  };
  [ClientMessageKind.SyncFiles]: {
    action: ClientMessageKind.SyncFiles;
  };
  [ClientMessageKind.Lsp]: {
    action: ClientMessageKind.Lsp;
    message: Record<string, unknown>;
  };
}[S];
