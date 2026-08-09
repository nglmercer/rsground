import { createSignal, Show } from "solid-js";

import { TextField } from "@components/TextField";
import { ErrorIcon } from "@icons/ErrorIcon";
import { LockIcon } from "@icons/Lock";

import { fetchProject, setProject } from "../services";
import { projectInfo } from "../stores";

import styles from "./RequestPassword.module.sass";
import { ApiErrorMessage, HttpStatus } from "@constants";

export function RequestPassword() {
  const [hint, setHint] = createSignal("");
  const [password, setPassword] = createSignal("");

  return (
    <div class={styles.container}>
      <h2>This project needs password</h2>
      <TextField
        onInput={(ev) => {
          setHint("");
          setPassword(ev.currentTarget.value);
        }}
        beforeIcon={<LockIcon />}
        placeholder="Enter password"
      />
      <Show when={hint()}>
        <span class={styles.error}>
          <ErrorIcon /> {hint()}
        </span>
      </Show>
      <button
        onClick={() => {
          if (!password()) {
            setHint("Put some text :)");
          } else {
            fetchProject(projectInfo.id, password()).then((project) => {
              setProject(project, false);
            }).catch(
              (err) => {
                if (
                  err instanceof Array && err[0] == HttpStatus.Unauthorized &&
                  (err[1] as string).includes(ApiErrorMessage.InvalidPassword)
                ) setHint(ApiErrorMessage.InvalidPassword);
              },
            );
          }
        }}
      >
        Submit
      </button>
    </div>
  );
}
