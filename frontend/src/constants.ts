export const StorageKey = {
  Auth: "auth",
  RedirectUrl: "redirect_url",
  ThemeMode: "themeMode",
} as const;

export const Route = {
  Root: "/",
  Auth: "/auth",
  AuthCallback: "/auth/callback",
  Fork: "fork",
} as const;

export const ApiPath = {
  Auth: "/auth",
  AuthCallback: "/auth/callback",
  AuthGuest: "/auth/guest",
  AuthMe: "/auth/me",
  CreateProject: "/create",
  ForkProject: "/fork",
  Project: "/project",
} as const;

export const HttpHeader = {
  Authorization: "Authorization",
  ContentType: "Content-Type",
  ProjectPassword: "X-Project-Password",
} as const;

export const HttpValue = {
  JsonContentType: "application/json",
  IncludeCredentials: "include",
} as const;

export const QueryParam = {
  Code: "code",
  State: "state",
} as const;

export const HttpMethod = {
  Get: "GET",
  Post: "POST",
} as const;

export const HttpStatus = {
  Unauthorized: 401,
  NotFound: 404,
} as const;

export const ProjectDefaults = {
  MainFile: "main.rs",
  Name: "Unnamed",
  ForkPath: "/fork",
} as const;

export const ProjectUserTuple = {
  NameIndex: 0,
  AccessIndex: 1,
} as const;

export const ProjectInfoField = {
  Id: "id",
  Users: "users",
  Requests: "requests",
  IsPublic: "is_public",
} as const;

export const ApiErrorMessage = {
  InvalidToken: "Invalid token",
  InvalidPassword: "Invalid password",
} as const;

export const ThemeDataAttribute = {
  Light: "light",
  Dark: "dark",
} as const;

export const ThemeSelectorConfig = {
  InputName: "theme-selector",
} as const;

export const WebSocketConfig = {
  Path: "/ws/",
  Ping: "ping",
  HeartbeatIntervalMs: 5_000,
  AbnormalClosureCode: 1006,
  AuthProtocolPrefix: "auth.",
} as const;

export const Panel = {
  Code: "code",
  Output: "output",
  FilePrefix: "file:",
  OutputTitle: "Output",
} as const;

export const DockviewConfig = {
  ThemeName: "rsground",
  ThemeClassName: "rsground-dockview",
  GapPx: 10,
  OverlayMounting: "absolute",
  OverlayGroup: "group",
  SingleTabMode: "default",
  OutputInitialHeight: 20,
  OutputMinimumHeight: 50,
  OutputDirection: "below",
  CodeDirection: "above",
} as const;

export const EditingFileField = {
  SyncedRevision: "synced_revision",
  EditorOpen: "editor_open",
} as const;

export const ContextMenuLevel = {
  Error: "error",
  Warning: "warning",
} as const;

export type ContextMenuLevel =
  typeof ContextMenuLevel[keyof typeof ContextMenuLevel];

export const ToastKind = {
  Debug: "debug",
  Info: "info",
  Success: "success",
  Warning: "warn",
  Error: "error",
} as const;

export type ToastKind = typeof ToastKind[keyof typeof ToastKind];

export const ToastIcon = {
  Info: "info",
  Success: "success",
  Warning: "warning",
  Error: "error",
} as const;

export const ThemeAppearance = {
  Auto: "auto",
  Dark: "dark",
  Light: "light",
} as const;

export const UiValue = {
  SidebarBreakpointPx: 750,
  OutputChunkSize: 1024,
  ToastDurationMs: 5_000,
  AccessRequestToastDurationMs: 5_000,
  ProjectNotFoundToastDurationMs: 2_000,
  ProjectPasswordDebounceMs: 500,
  CursorHueDegrees: 360,
  TooltipOpenDelayMs: 200,
  OutputScrollThresholdPx: 50,
  CursorHashOffset: 1,
} as const;

export const RandomName = {
  NumberRange: 999,
} as const;

export const Sync = {
  Annotation: "SYNC",
} as const;

export const SelectFieldConfig = {
  DefaultPrefix: "select-",
  DefaultPlacement: "right-start",
} as const;

export const ContextMenuConfig = {
  Placement: "right-start",
  CursorAnchorOffsetPx: 16,
  CursorLeftOffsetPx: 8,
} as const;

export const BACKEND_HOST =
  import.meta.env.VITE_BACKEND_HOST || "http://localhost:8080";
