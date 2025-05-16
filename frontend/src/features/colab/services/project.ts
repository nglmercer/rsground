import { batch, untrack } from "solid-js";
import SWAL from "sweetalert2";

import { authInfo } from "@features/auth/stores";
import { onWsMessage, startWebsocket } from "@features/ws/services";
import { AccessLevel, ServerMessageKind } from "@features/ws/types";
import { BACKEND_HOST } from "@services";
import { showModal } from "@services/modal";
import { showToast } from "@services/toast";

import { ProjectInfo } from "../types";
import {
  projectInfo,
  setIsProjectOwner,
  setProjectAccess,
  setProjectInfo,
} from "../stores";
import { WaitingAccess } from "../views";

onWsMessage(ServerMessageKind.UpdateAccess, (msg) => {
  setProjectInfo("users", msg.user_id, 1, msg.access);

  if (msg.user_id === untrack(authInfo)?.id) {
    setProjectAccess(msg.access);

    if (msg.access === AccessLevel.Editor) {
      projectInfo.requests[msg.user_id] &&
        setProjectInfo("requests", msg.user_id, undefined);
      showToast("success", {
        titleText: "You have been granted to edit",
      });
    } else if (msg.access === AccessLevel.ReadOnly) {
      projectInfo.requests[msg.user_id] &&
        setProjectInfo("requests", msg.user_id, undefined);
      showToast("success", {
        titleText: "You have been granted to read",
      });
    } else if (msg.access === AccessLevel.Queue) {
      projectInfo.requests[msg.user_id] &&
        setProjectInfo("requests", msg.user_id, undefined);
      showToast("error", {
        titleText: "You have been kicked",
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

onWsMessage(ServerMessageKind.RequestAccess, (msg) => {
  setProjectInfo("requests", msg.user_id, msg.user_name);
});

export function setProject(project: ProjectInfo) {
  // Check if has access to project
  if (project.users == null) {
    // TODO: Pending permission, listen to permission granted.
    // Once user is allowed, should restart websocket connection
    // for receive welcome
    setProjectInfo("id", project.id);
    showModal(WaitingAccess, {
      allowOutsideClick: false,
    });
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
