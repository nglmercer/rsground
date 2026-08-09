import { batch, untrack } from "solid-js";
import SWAL from "sweetalert2";

import { authInfo } from "@features/auth/stores";
import { onWsMessage, startWebsocket } from "@features/ws/services";
import { AccessLevel, ServerMessageKind } from "@features/ws/types";
import { BACKEND_HOST } from "@services";
import { showModal } from "@services/modal";
import { showToast } from "@services/toast";
import {
  ApiPath,
  HttpHeader,
  HttpMethod,
  HttpStatus,
  ProjectDefaults,
  ProjectInfoField,
  ProjectUserTuple,
  ToastKind,
  UiValue,
} from "@constants";

import { ProjectInfo } from "../types";
import {
  isProjectOwner,
  projectAccess,
  projectInfo,
  setIsProjectOwner,
  setProjectAccess,
  setProjectInfo,
} from "../stores";
import { WaitingAccess } from "../views";

export class ProjectRequestError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
  ) {
    super(body || `Project request failed (${status})`);
    this.name = "ProjectRequestError";
  }
}

export type ProjectSetupResult = "ready" | "redirecting";

onWsMessage(ServerMessageKind.UpdateAccess, (msg) => {
  if (!isProjectOwner()) {
    const oldAccess = untrack(projectAccess);

    setProjectAccess(msg.access);

    if (oldAccess === AccessLevel.Queue && msg.access !== AccessLevel.Queue) {
      window.location.reload();
      return;
    }

    if (msg.access === AccessLevel.Editor) {
      showToast(ToastKind.Success, {
        titleText: "You have been granted to edit",
      });
    } else if (msg.access === AccessLevel.ReadOnly) {
      showToast(ToastKind.Success, {
        titleText: "You have been granted to read",
      });
    } else if (msg.access === AccessLevel.Queue) {
      showToast(ToastKind.Error, {
        titleText: "You have been kicked",
      });
    }
    return;
  }

  setProjectInfo(
    ProjectInfoField.Users,
    msg.user_id,
    ProjectUserTuple.AccessIndex,
    msg.access,
  );

  projectInfo.requests[msg.user_id] &&
    setProjectInfo(ProjectInfoField.Requests, msg.user_id, undefined);
});

onWsMessage(ServerMessageKind.ProjectConfig, (msg) => {
  setProjectInfo({
    name: msg.name,
    is_public: msg.is_public,
  });
});

onWsMessage(ServerMessageKind.RequestAccess, (msg) => {
  showToast(ToastKind.Info, {
    titleText: `${msg.user_name} is requesting access`,
    timer: UiValue.AccessRequestToastDurationMs,
  }).then(() => {
    setProjectInfo(ProjectInfoField.Requests, msg.user_id, msg.user_name);
  });
});

export async function setProject(
  project: ProjectInfo,
  shouldFork: boolean,
): Promise<ProjectSetupResult> {
  // Check if has access to project
  if (project.users == null) {
    // TODO: Pending permission, listen to permission granted.
    // Once user is allowed, should restart websocket connection
    // for receive welcome
    setProjectInfo(ProjectInfoField.Id, project.id);
    showModal(WaitingAccess, {
      allowOutsideClick: false,
    });
    return "ready";
  }

  if (shouldFork) {
    await forkProject(project.id);
    return "redirecting";
  }

  batch(() => {
    let isOwner = project.owner === untrack(authInfo).id;
    setIsProjectOwner(isOwner);

    setProjectAccess(
      isOwner
        ? AccessLevel.Editor
        : project.users[untrack(authInfo).id]?.[ProjectUserTuple.AccessIndex] ??
          AccessLevel.Queue,
    );
    setProjectInfo(project);
  });

  // Close current modal, maybe it is password
  // or waiting screen
  SWAL.close();

  startWebsocket();

  return "ready";
}

export async function createProject(
  owner: string,
  name: string = ProjectDefaults.Name,
): Promise<string> {
  let res = await fetch(`${BACKEND_HOST}${ApiPath.CreateProject}/${encodeURIComponent(name)}`, {
    method: HttpMethod.Post,
    headers: {
      [HttpHeader.Authorization]: `Bearer ${owner}`,
    },
  });

  const body = await res.text();

  if (!res.ok) throw new ProjectRequestError(res.status, body);

  let projectId: unknown;
  try {
    projectId = JSON.parse(body).id;
  } catch {
    throw new Error("The server returned an invalid project.");
  }

  if (typeof projectId !== "string" || !projectId) {
    throw new Error("The server did not return a project id.");
  }

  return projectId;
}

export async function fetchProject(
  project_id: string,
  password = "",
): Promise<ProjectInfo> {
  let res = await fetch(`${BACKEND_HOST}${ApiPath.Project}/${project_id}`, {
    method: HttpMethod.Get,
    headers: {
      [HttpHeader.Authorization]: `Bearer ${untrack(authInfo)?.jwt}`,
      ...(password ? { [HttpHeader.ProjectPassword]: password } : {}),
    },
  });

  const body = await res.text();

  if (res.status === HttpStatus.Unauthorized) {
    try {
      return JSON.parse(body);
    } catch {}
  }

  if (!res.ok) throw new ProjectRequestError(res.status, body);

  try {
    return JSON.parse(body);
  } catch {
    throw new Error("The server returned an invalid project.");
  }
}

export async function forkProject(project_id: string): Promise<ProjectSetupResult> {
  let res = await fetch(`${BACKEND_HOST}${ApiPath.ForkProject}/${project_id}`, {
    method: HttpMethod.Post,
    headers: {
      [HttpHeader.Authorization]: `Bearer ${untrack(authInfo)?.jwt}`,
    },
  });

  const body = await res.text();

  if (!res.ok) throw new ProjectRequestError(res.status, body);

  let projectId: unknown;
  try {
    projectId = JSON.parse(body).id;
  } catch {
    throw new Error("The server returned an invalid fork.");
  }

  if (typeof projectId !== "string" || !projectId) {
    throw new Error("The server did not return a fork id.");
  }

  redirectToProject(projectId);
  return "redirecting";
}

export function redirectToProject(projectId: string): void {
  window.location.replace(`/${encodeURIComponent(projectId)}`);
}
