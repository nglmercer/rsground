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

import {
  createProject,
  fetchProject,
  ProjectRequestError,
  ProjectSetupResult,
  redirectToProject,
  setProject,
} from "../services";
import { RequestPassword } from "../views";
import { setProjectInfo } from "../stores";

export async function interceptProjectRoutes(): Promise<ProjectSetupResult> {
  const pathname = window.location.pathname;

  if (pathname === Route.Root) {
    return createProjectWith(untrack(authInfo));
  }

  if (pathname === Route.Auth || pathname.startsWith(`${Route.Auth}/`)) {
    window.location.replace(Route.Root);
    return "redirecting";
  }

  const segments = pathname.split("/").filter(Boolean);
  const projectId = segments[0];
  const maybeAction = segments[1];

  if (
    !projectId ||
    segments.length > 2 ||
    (maybeAction && maybeAction !== Route.Fork)
  ) {
    throw new Error("This project URL is not valid.");
  }

  try {
    const project = await fetchProject(projectId);
    return await setProject(project, maybeAction === Route.Fork);
  } catch (error) {
    if (error instanceof ProjectRequestError) {
      if (error.status === HttpStatus.NotFound) {
        void showToast(ToastKind.Error, {
          titleText: "Project not found. Creating a new one",
          timer: UiValue.ProjectNotFoundToastDurationMs,
        });
        return createProjectWith(untrack(authInfo));
      }

      if (error.status === HttpStatus.Unauthorized) {
        if (error.body.includes(ApiErrorMessage.InvalidToken)) {
          // Retrying synchronously here used to create an infinite recursion
          // when a token expired between /auth/me and this request. Surface a
          // recoverable error instead; the app-level retry re-checks auth.
          throw new Error("Your session expired. Please try again.");
        }

        setProjectInfo(ProjectInfoField.Id, projectId);
        void showModal(RequestPassword);
        return "ready";
      }
    }

    throw error instanceof Error
      ? error
      : new Error("Unable to load this project. Please try again.");
  }
}

async function createProjectWith(
  authInfo: AuthInfo | null,
): Promise<ProjectSetupResult> {
  if (!authInfo?.jwt) {
    throw new Error("Your session is not ready. Please try again.");
  }

  const projectId = await createProject(authInfo.jwt);
  // Replace the root/new-project URL so Back does not create another project.
  redirectToProject(projectId);
  return "redirecting";
}
