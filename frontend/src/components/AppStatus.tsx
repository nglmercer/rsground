import { Show } from "solid-js";

import { Spinner } from "./Spinner";

import styles from "./AppStatus.module.sass";

export type AppStatusKind = "loading" | "redirecting" | "error";

export interface AppStatusProps {
  kind: AppStatusKind;
  title: string;
  message: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export function AppStatus(props: AppStatusProps) {
  const isError = () => props.kind === "error";

  return (
    <main
      class={styles.container}
      role={isError() ? "alert" : "status"}
      aria-live="polite"
      aria-busy={!isError()}
    >
      <div class={styles.card}>
        <Show
          when={!isError()}
          fallback={<div class={styles.error_icon} aria-hidden="true">!</div>}
        >
          <Spinner
            class={styles.spinner}
            width="2.5rem"
            height="2.5rem"
            aria-hidden="true"
          />
        </Show>

        <h1>{props.title}</h1>
        <p>{props.message}</p>

        <Show when={props.action}>
          {(action) => (
            <button type="button" onClick={action().onClick}>
              {action().label}
            </button>
          )}
        </Show>
      </div>
    </main>
  );
}
