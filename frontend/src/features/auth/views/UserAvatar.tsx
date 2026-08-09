import { createMemo, Show } from "solid-js";
import { SolidUserIcon } from "@icons/SolidUser";

import { authInfo } from "../stores";

import styles from "./UserAvatar.module.sass";

export function RawUserAvatar() {
  const has_avatar = createMemo(() => !!authInfo()?.avatar_url);

  return (
    <>
      <Show when={has_avatar()}>
        <img
          class={styles.raw_avatar_img}
          src={authInfo()?.avatar_url}
          alt={`${authInfo()?.name ?? "User"} avatar`}
        />
      </Show>
      <Show when={!has_avatar()}>
        <SolidUserIcon height="100%" width="100%" aria-hidden="true" />
      </Show>
    </>
  );
}
