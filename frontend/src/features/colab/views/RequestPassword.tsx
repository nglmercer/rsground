import { createSignal, Show } from "solid-js";

import { Spinner } from "@components/Spinner";
import { TextField } from "@components/TextField";
import { ErrorIcon } from "@icons/ErrorIcon";
import { LockIcon } from "@icons/Lock";

import { fetchProject, ProjectRequestError, setProject } from "../services";
import { projectInfo } from "../stores";

import styles from "./RequestPassword.module.sass";
import { ApiErrorMessage, HttpStatus } from "@constants";

export function RequestPassword() {
  const [hint, setHint] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [isSubmitting, setIsSubmitting] = createSignal(false);

  const submit = async () => {
    if (isSubmitting()) return;

    setHint("");
    if (!password()) {
      setHint("Enter a password first.");
      return;
    }

    setIsSubmitting(true);
    try {
      const project = await fetchProject(projectInfo.id, password());
      await setProject(project, false);
    } catch (error) {
      if (
        error instanceof ProjectRequestError &&
        error.status === HttpStatus.Unauthorized &&
        error.body.includes(ApiErrorMessage.InvalidPassword)
      ) {
        setHint(ApiErrorMessage.InvalidPassword);
      } else {
        console.error("Unable to open password-protected project:", error);
        setHint("Unable to open the project. Please try again.");
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div class={styles.container}>
      <h2>This project needs password</h2>
      <TextField
        disabled={isSubmitting()}
        value={password()}
        onInput={(ev) => {
          setHint("");
          setPassword(ev.currentTarget.value);
        }}
        onKeyDown={(ev) => {
          if (ev.key === "Enter") void submit();
        }}
        beforeIcon={<LockIcon />}
        placeholder="Enter password"
      />
      <Show when={hint()}>
        <span class={styles.error}>
          <ErrorIcon /> {hint()}
        </span>
      </Show>
      <button disabled={isSubmitting()} aria-busy={isSubmitting()} onClick={() => void submit()}>
        <Show when={!isSubmitting()} fallback={<Spinner aria-label="Checking password" />}>
          Submit
        </Show>
      </button>
    </div>
  );
}
