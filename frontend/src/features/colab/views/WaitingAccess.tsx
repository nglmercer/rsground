import { createEffect, createSignal, Show } from "solid-js";
import SWAL from "sweetalert2";

import { loginGithub } from "@features/auth/services";
import { authInfo } from "@features/auth/stores";
import { AccessLevel } from "@features/ws/types";
import { BrandsGithubIcon } from "@icons/BrandsGithub";

import { projectAccess } from "../stores";
import { createProject, redirectToProject } from "../services";
import { ToastKind } from "@constants";
import { showToast } from "@services/toast";

import styles from "./WaitingAccess.module.sass";

export function WaitingAccess() {
  const [isCreating, setIsCreating] = createSignal(false);

  createEffect(() => {
    if (projectAccess() !== AccessLevel.Queue) {
      SWAL.close();
    }
  });
  return (
    <div class={styles.container}>
      <h2>Project is private</h2>
      <span>Wait for your access request to be accepted.</span>
      <p class={styles.username_paragraph}>
        Your name is: <span class={styles.username}>{authInfo().name}</span>
      </p>
      <div class={styles.actions}>
        <button
          disabled={isCreating()}
          aria-busy={isCreating()}
          onClick={async () => {
            if (isCreating()) return;

            setIsCreating(true);
            try {
              const newProject = await createProject(authInfo().jwt);
              redirectToProject(newProject);
            } catch (error) {
              console.error("Unable to create a project:", error);
              void showToast(ToastKind.Error, {
                titleText: "Could not create a project",
                text: "Please check your connection and try again.",
              });
            } finally {
              setIsCreating(false);
            }
          }}
        >
          <Show when={!isCreating()} fallback="Creating…">
            Create new
          </Show>
        </button>
        <button
          class={styles.action_secondary}
          onClick={() => {
            loginGithub();
          }}
        >
          <BrandsGithubIcon width="1rem" height="1rem"/>
          Log In
        </button>
      </div>
    </div>
  );
}
