import { For } from "solid-js/web";

import { onWsMessage, sendMessage } from "@features/ws/services";
import { projectAccess } from "@features/colab/stores";
import {
  AccessLevel,
  ClientMessageKind,
  ServerMessageKind,
} from "@features/ws/types";
import { PlayIcon } from "@icons/Play";
import { SkullIcon } from "@icons/Skull";

import { ansiToHtml } from "../utils";
import { outputPanel, setOutputPanel } from "../stores";

import styles from "./OutputPanel.module.sass";
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { ProcessExitCode, UiValue } from "@constants";

const decoder = new TextDecoder();

export function OutputPanel() {
  let contentRef: HTMLDivElement;

  let [exitCode, setExitCode] = createSignal<number | null>(null);
  const canEdit = () => projectAccess() === AccessLevel.Editor;

  const unsubscribeOutputStart = onWsMessage(ServerMessageKind.SyncOutputStart, () => {
    setOutputPanel([]);
    setExitCode(null);
    decoder.decode();
  });

  const unsubscribeOutput = onWsMessage(ServerMessageKind.SyncOutput, (msg) => {
    const decoded = decoder.decode(new Uint8Array(msg.buf), { stream: true });

    if (
      outputPanel.length === 0 ||
      outputPanel[outputPanel.length - 1].length >= UiValue.OutputChunkSize
    ) {
      setOutputPanel(outputPanel.length, decoded);
    } else {
      setOutputPanel(outputPanel.length - 1, (prev) => prev + decoded);
    }
  });

  const unsubscribeOutputEnd = onWsMessage(ServerMessageKind.SyncOutputEnd, (msg) => {
    const remainder = decoder.decode();
    if (remainder) {
      if (outputPanel.length === 0) {
        setOutputPanel(0, remainder);
      } else {
        setOutputPanel(outputPanel.length - 1, (prev) => prev + remainder);
      }
    }
    setExitCode(msg.exit_code)
  });

  onCleanup(() => {
    unsubscribeOutputStart();
    unsubscribeOutput();
    unsubscribeOutputEnd();
  });

  return (
    <div class={styles.container}>
      <ul class={styles.actions} aria-label="Output actions">
        <li>
          <button
            type="button"
            class={`${styles.action} ${styles.action_play}`}
            aria-label="Run code"
            title="Run code"
            disabled={!canEdit()}
            aria-disabled={!canEdit()}
            onClick={() => sendMessage(ClientMessageKind.Execute, {})}
          >
            <PlayIcon aria-hidden="true" />
          </button>
        </li>

        <li>
          <button
            type="button"
            class={`${styles.action} ${styles.action_kill}`}
            aria-label="Kill program"
            title="Kill program"
            disabled={!canEdit()}
            aria-disabled={!canEdit()}
            onClick={() => sendMessage(ClientMessageKind.StopExecute, {})}
          >
            <SkullIcon aria-hidden="true" />
          </button>
        </li>

        <Show when={exitCode() == ProcessExitCode.Success}>
          <li>
            Successfully
          </li>
        </Show>
        {/* Non-zero and non-null */}
        <Show when={!!exitCode()}>
          <li>
            Exit status: {exitCode()}
          </li>
        </Show>
      </ul>

      <div ref={contentRef} class={styles.output}>
        <For each={outputPanel}>
          {(buf) => {
            onMount(() => {
              const target = contentRef.scrollHeight - contentRef.clientHeight;

              if (
                contentRef.scrollTop + UiValue.OutputScrollThresholdPx >= target
              ) {
                contentRef.scrollTop = target;
              }
            });
            return ansiToHtml(buf);
          }}
        </For>
      </div>
    </div>
  );
}
