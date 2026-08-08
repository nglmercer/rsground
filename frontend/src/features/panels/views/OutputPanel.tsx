import { For } from "solid-js/web";

import { onWsMessage, sendMessage } from "@features/ws/services";
import { ClientMessageKind, ServerMessageKind } from "@features/ws/types";
import { PlayIcon } from "@icons/Play";
import { SkullIcon } from "@icons/Skull";

import { ansiToHtml } from "../utils";
import { outputPanel, setOutputPanel } from "../stores";

import styles from "./OutputPanel.module.sass";
import { createSignal, onMount, Show } from "solid-js";

const decoder = new TextDecoder();

export function OutputPanel() {
  let contentRef: HTMLDivElement;

  let [exitCode, setExitCode] = createSignal<number | null>(null);

  onWsMessage(ServerMessageKind.SyncOutputStart, () => {
    setOutputPanel([]);
    setExitCode(null);
    decoder.decode();
  });

  onWsMessage(ServerMessageKind.SyncOutput, (msg) => {
    const decoded = decoder.decode(new Uint8Array(msg.buf), { stream: true });

    if (
      outputPanel.length === 0 ||
      outputPanel[outputPanel.length - 1].length >= 1024
    ) {
      setOutputPanel(outputPanel.length, decoded);
    } else {
      setOutputPanel(outputPanel.length - 1, (prev) => prev + decoded);
    }
  });

  onWsMessage(ServerMessageKind.SyncOutputEnd, (msg) => {
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

  return (
    <div class={styles.container}>
      <ul class={styles.actions} aria-label="Output actions">
        <li
          class={styles.action_play}
          aria-role="button"
          aria-label="Run code"
          title="Run code"
          onClick={() => sendMessage(ClientMessageKind.Execute, {})}
        >
          <PlayIcon />
        </li>

        <li
          class={styles.action_kill}
          aria-role="button"
          aria-label="Kill program"
          title="Kill program"
          onClick={() => sendMessage(ClientMessageKind.StopExecute, {})}
        >
          <SkullIcon />
        </li>

        <Show when={exitCode() == 0}>
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

              if (contentRef.scrollTop + 50 >= target) {
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
