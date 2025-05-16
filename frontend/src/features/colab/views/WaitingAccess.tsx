import { createEffect } from "solid-js";
import SWAL from "sweetalert2";

import { loginGithub } from "@features/auth/services";
import { authInfo } from "@features/auth/stores";
import { AccessLevel } from "@features/ws/types";
import { BrandsGithubIcon } from "@icons/BrandsGithub";

import { projectAccess } from "../stores";
import { createProject } from "../services";

import styles from "./WaitingAccess.module.sass";

export function WaitingAccess() {
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
          onClick={async () => {
            let newProject = await createProject(authInfo().jwt);

            location.pathname = "/" + newProject;
          }}
        >
          Create new
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
