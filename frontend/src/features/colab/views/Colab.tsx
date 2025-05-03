import Dialog from "@corvu/dialog";
import { createEffect, createSignal, For, Show, untrack } from "solid-js";

import { SelectField } from "@components/SelectField";
import { Switchbox } from "@components/Switchbox";
import { TextField } from "@components/TextField";
import { AccessLevel, ClientMessageKind } from "@features/ws/types";
import { sendMessage } from "@features/ws/services";
import { LockIcon } from "@icons/Lock";
import { BrandsGithubIcon } from "@icons/BrandsGithub";
import { showToast } from "@services/toast";

import {
  isColabOpen,
  isProjectOwner,
  projectId,
  projectInfo,
  setIsColabOpen,
  setProjectInfo,
} from "../stores";

import styles from "./Colab.module.sass";

export function Colab() {
  const requestUsers = ["CHIWO", "Jopzgo", "gg0074x", "Otro"];

  return (
    <Dialog open={isColabOpen()} onOpenChange={setIsColabOpen}>
      <Dialog.Portal>
        <Dialog.Overlay class={styles.overlay} />
        <Dialog.Content class={styles.content}>
          <h2 class={styles.title}>Live Collaboration</h2>

          <div class={styles.container}>
            <div>
              <h3 class={styles.subtitle}>Room settings</h3>

              <ColabPublicPassword />
              <ColabButtons />
            </div>

            <div>
              <h3 class={styles.subtitle}>Members</h3>

              <TextField
                beforeIcon={<BrandsGithubIcon />}
                placeholder="Username"
              />

              <ul class={styles.user_list}>
                <For each={Object.entries(projectInfo()?.users ?? {})}>
                  {([user_id, [username, access]]) => (
                    <li class={styles.member}>
                      <span class={styles.member_name}>
                        {username}
                      </span>

                      <SelectField
                        value={access}
                        disabled={!isProjectOwner()}
                        options={[AccessLevel.Editor, AccessLevel.ReadOnly]}
                        onValueChange={(access) => {
                          sendMessage(ClientMessageKind.PermitAccess, {
                            user_id,
                            access,
                          });
                        }}
                      />
                    </li>
                  )}
                </For>
              </ul>

              <h3 class={styles.subtitle}>Pending Requests</h3>
              <ul class={styles.user_list}>
                <For each={requestUsers}>
                  {(name) => (
                    <li class={styles.member}>
                      <span class={styles.member_name}>
                        {name}
                      </span>

                      <ul class={styles.button_group}>
                        <button class={styles.success}>Allow</button>
                        <button class={styles.error}>Kick</button>
                      </ul>
                    </li>
                  )}
                </For>
              </ul>
            </div>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog>
  );
}

function ColabPublicPassword() {
  const [password, setPassword] = createSignal("");

  let debounce: NodeJS.Timeout;
  let first_time = true;
  createEffect(() => {
    let pass = password();

    if (first_time && (first_time = false, true)) return;

    if (debounce) clearTimeout(debounce);

    debounce = setTimeout(() => {
      sendMessage(ClientMessageKind.Config, { password: pass });
    }, 500);
  });

  return (
    <>
      <label class={styles.checkbox_input}>
        Public room
        <Switchbox
          checked={projectInfo()?.is_public ?? false}
          onChange={(ev) => {
            let projectInfo_ = untrack(projectInfo);
            if (!!projectInfo_) {
              setProjectInfo({
                ...projectInfo_,
                is_public: ev.currentTarget.checked,
              });
              sendMessage(ClientMessageKind.Config, {
                is_public: ev.currentTarget.checked,
              });
            }
          }}
        />
      </label>

      <Show when={projectInfo()?.is_public ?? false}>
        <TextField
          value={projectInfo()?.password ?? ""}
          onInput={(ev) => setPassword(ev.currentTarget.value)}
          beforeIcon={<LockIcon />}
          placeholder="Leave empty for no password"
        />
        <span class={styles.password_hint}>Password is visible</span>
      </Show>
    </>
  );
}

function ColabButtons() {
  const copyPath = (suffix = "") => {
    navigator.clipboard.writeText(
      `${location.protocol}//${location.host}/${projectId()}${suffix}`,
    );
  };

  return (
    <div class={styles.buttons_container}>
      <button onClick={() => copyPath()}>
        Copy colab link
      </button>
      <button onClick={() => copyPath("/fork")}>
        Copy fork link
      </button>
      <button
        onClick={() => {
          showToast("debug", {
            titleText: "Fork project",
            text: "Not implemented yet",
          });
        }}
      >
        Fork
      </button>
    </div>
  );
}
