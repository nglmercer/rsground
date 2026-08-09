import { untrack } from "solid-js";

import { authInfo } from "@features/auth/stores";
import { AuthInfo } from "@features/auth/types";
import { showModal } from "@services/modal";
import { showToast } from "@services/toast";
import {
  HttpStatus,
  ApiErrorMessage,
  ProjectInfoField,
  Route,
  ToastKind,
  UiValue,
} from "@constants";

import { createProject, fetchProject, setProject } from "../services";
import { RequestPassword } from "../views";
import { setProjectInfo } from "../stores";

export function interceptProjectRoutes() {
  if (window.location.pathname === Route.Root) {
    createProjectWith(untrack(authInfo));
    return;
  }

  let segments = window.location.pathname.split("/");
  segments.shift();

  let projectId = segments.shift();
  let maybeAction = segments.shift();

  fetchProject(projectId).then((project) => {
    setProject(project, maybeAction === Route.Fork);
  }).catch((err: [number, string]) => {
    if (err instanceof Array) {
      if (err[0] === HttpStatus.NotFound) {
        showToast(ToastKind.Error, {
          titleText: "Project not found. Creating new one",
          timer: UiValue.ProjectNotFoundToastDurationMs,
        }).then(() => {
          createProjectWith(untrack(authInfo));
        });
        return;
      }

      // Just retry until auto-logged
      if (
        err[0] === HttpStatus.Unauthorized &&
        err[1] == ApiErrorMessage.InvalidToken
      ) {
        interceptProjectRoutes();
        return;
      }

      if (err[0] == HttpStatus.Unauthorized) {
        setProjectInfo(ProjectInfoField.Id, projectId);
        showModal(RequestPassword);
        return;
      }
    }

    console.error(err);
    showToast(ToastKind.Error, {
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
