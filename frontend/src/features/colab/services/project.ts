import { untrack } from "solid-js/web";

import { authInfo } from "@features/auth/stores";
import { onWsMessage } from "@features/ws/services";
import { AccessLevel, ServerMessageKind } from "@features/ws/types";
import { BACKEND_HOST } from "@services";
import { showToast } from "@services/toast";

import { ProjectInfo } from "../types";
import { projectInfo, setProjectAccess, setProjectInfo } from "../stores";

onWsMessage(ServerMessageKind.UpdateAccess, (msg) => {
  setProjectInfo((projectInfo) => ({
    ...projectInfo,
    users: {
      ...projectInfo.users,
      [msg.user_id]: [
        projectInfo.users[msg.user_id]?.[0] ?? "Unknown",
        msg.access,
      ],
    },
  }));

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
  const project_info = untrack(projectInfo);
  if (project_info) {
    setProjectInfo({
      ...project_info,
      name: msg.name,
      is_public: msg.is_public,
      password: msg.password,
    });
  }
});

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
  let res = await fetch(`${BACKEND_HOST}/project/${project_id}?${password}`, {
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
