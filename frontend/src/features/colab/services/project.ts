import { batch, untrack } from "solid-js";
import SWAL from "sweetalert2";

import { authInfo } from "@features/auth/stores";
import { onWsMessage, startWebsocket } from "@features/ws/services";
import { AccessLevel, ServerMessageKind } from "@features/ws/types";
import { BACKEND_HOST } from "@services";
import { showModal } from "@services/modal";
import { showToast } from "@services/toast";

import { ProjectInfo } from "../types";
import { setIsProjectOwner, setProjectAccess, setProjectInfo } from "../stores";
import { WaitingAccess } from "../views";

onWsMessage(ServerMessageKind.UpdateAccess, (msg) => {
  setProjectInfo("users", msg.user_id, 1, msg.access);

  if (msg.user_id === untrack(authInfo)?.id) {
    setProjectAccess(msg.access);
    if (msg.access === AccessLevel.Editor) {
      showToast("success", {
        titleText: "You have been granted to edit",
      });
    } else if (msg.access === AccessLevel.ReadOnly) {
      showToast("success", {
        titleText: "You have been granted to read",
      });
    }
  }
});

onWsMessage(ServerMessageKind.ProjectConfig, (msg) => {
  setProjectInfo({
    name: msg.name,
    is_public: msg.is_public,
    password: msg.password,
  });
});

export function setProject(project: ProjectInfo) {
  // Check if has access to project
  if (project.users == null) {
    // TODO: Pending permission, listen to permission granted.
    // Once user is allowed, should restart websocket connection
    // for receive welcome
    setProjectInfo("id", project.id);
    return;
  }

  batch(() => {
    if (project.owner === untrack(authInfo).id) {
      setIsProjectOwner(true);
    }

    setProjectAccess(
      project.users[untrack(authInfo).id]?.[1] ?? AccessLevel.Queue,
    );
    setProjectInfo(project);
  });

  // Close current modal, maybe it is password
  // or waiting screen
  SWAL.close();

  startWebsocket();
}

export async function createProject(
  owner: string,
  name: string = "Unnamed",
): Promise<string> {
  let res = await fetch(`${BACKEND_HOST}/create/${name}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${owner}`,
    },
  });

  if (!res.ok) {
    return null;
  }

  return (await res.json()).id;
}

export async function fetchProject(
  project_id: string,
  password = "",
): Promise<ProjectInfo> {
  let res = await fetch(`${BACKEND_HOST}/project/${project_id}?p=${password}`, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${untrack(authInfo)?.jwt}`,
    },
  });

  const body = await res.text();

  if (res.status === 401) {
    try {
      return JSON.parse(body);
    } catch {}
  }

  if (!res.ok) {
    throw [res.status, body];
  }

  return JSON.parse(body);
}
