import { observable, untrack } from "solid-js";

import { authInfo } from "@features/auth/stores";
import { AuthInfo } from "@features/auth/types";
import { showModal } from "@services/modal";
import { showToast } from "@services/toast";

import { createProject, fetchProject, setProject } from "../services";
import { setProjectId } from "../stores";
import { RequestPassword } from "../views";

export function interpectProjectRoutes() {
  if (window.location.pathname === "/") {
    observable(authInfo).subscribe(createProjectWith);
    return;
  }

  let segments = window.location.pathname.split("/");
  segments.shift();

  let projectId = segments.shift();
  let maybeAction = segments.shift();

  if (maybeAction === "fork") {
    // TODO: fork project
    showToast("debug", {
      titleText: "Fork project",
      text: "Not implemented yet",
    });
    return;
  }

  fetchProject(projectId).then(setProject).catch((err: [number, string]) => {
    if (err instanceof Array) {
      if (err[0] === 404) {
        showToast("error", {
          titleText: "Project not found. Creating new one",
          timer: 2_000,
        }).then(() => {
          createProjectWith(untrack(authInfo));
        });
        return;
      }

      // Just retry until auto-logged
      if (err[0] === 401 && err[1] == "Invalid token") {
        interpectProjectRoutes();
        return;
      }

      if (err[0] == 401) {
        setProjectId(projectId);
        showModal(RequestPassword);
        return;
      }
    }

    console.error(err);
    showToast("error", {
      titleText: "Unexpected error",
      text: "Contact to developers",
    });
  });
}

async function createProjectWith(authInfo: AuthInfo) {
  if (!!authInfo?.jwt) {
    let projectId = await createProject(authInfo.jwt);

    window.location.pathname = "/" + projectId;
  }
}
