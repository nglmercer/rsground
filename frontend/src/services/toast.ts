import { untrack } from "solid-js";
import SWAL, { SweetAlertOptions } from "sweetalert2";

import { ThemeMode, themeMode } from "@features/theme";
import { ThemeAppearance, ToastIcon, ToastKind, UiValue } from "@constants";

import styles from "./toast.module.sass";

const baseToast = SWAL.mixin({
  position: "bottom-right",
  toast: true,
  showConfirmButton: false,
  timer: UiValue.ToastDurationMs,
  timerProgressBar: true,
  didOpen(toast) {
    toast.onmouseenter = SWAL.stopTimer;
    toast.onmouseleave = SWAL.resumeTimer;
  },
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

export function showToast<T = any>(
  kind: ToastKind,
  options: SweetAlertOptions,
): ReturnType<typeof baseToast.fire<T>> {
  return baseToast.fire<T>({
    theme: untrack(themeMode) === ThemeMode.System
      ? ThemeAppearance.Auto
      : untrack(themeMode) === ThemeMode.Dark
      ? ThemeAppearance.Dark
      : ThemeAppearance.Light,
    icon: options.icon ||
      (kind === ToastKind.Debug
        ? ToastIcon.Info
        : kind === ToastKind.Success
        ? ToastIcon.Success
        : kind === ToastKind.Warning
        ? ToastIcon.Warning
        : kind === ToastKind.Error
        ? ToastIcon.Error
        : undefined),
    iconColor: "var(--icon-color)",
    customClass: {
      container: styles["container-" + kind],
      popup: styles.popup,
      ...options.customClass,
    },
    ...options,
  });
}
