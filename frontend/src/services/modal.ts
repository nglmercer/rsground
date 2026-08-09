import { JSX, untrack } from "solid-js";
import SWAL, { SweetAlertOptions } from "sweetalert2";

import { ThemeMode, themeMode } from "@features/theme";
import { ThemeAppearance } from "@constants";

import styles from "./modal.module.sass";
import { render } from "solid-js/web";

const baseToast = SWAL.mixin({
  position: "center",
  allowOutsideClick: false,
  showConfirmButton: false,
  showClass: {
    popup: styles.animation_show,
  },
  hideClass: {
    popup: styles.animation_hide,
  },
  customClass: {
    container: styles.container,
    popup: styles.popup,
  },
});

export function showModal<T = any>(
  content: () => JSX.Element,
  options: SweetAlertOptions = {}
): ReturnType<typeof baseToast.fire<T>> {
  const elem = document.createElement("div");
  elem.style.display = "contents";

  return baseToast.fire<T>({
    theme: untrack(themeMode) === ThemeMode.System
      ? ThemeAppearance.Auto
      : untrack(themeMode) === ThemeMode.Dark
      ? ThemeAppearance.Dark
      : ThemeAppearance.Light,
    ...options,
    html: elem,
    didOpen() {
      render(content, elem);
    }
  });
}
