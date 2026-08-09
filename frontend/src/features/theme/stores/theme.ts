import { observable } from "solid-js";
import { ThemeMode } from "../types";
import { createLocalStoredSignal } from "@utils/createLocalStoredSignal";
import { StorageKey, ThemeDataAttribute } from "@constants";

export const [themeMode, setThemeMode] = createLocalStoredSignal<ThemeMode>(
  StorageKey.ThemeMode,
  ThemeMode.System,
  (v) => ThemeMode[v],
  (v) => ThemeMode[v],
);

const htmlRoot = document.getElementsByTagName("html")[0];
observable(themeMode).subscribe((themeMode) => {
  switch (themeMode) {
    case ThemeMode.System:
      delete htmlRoot.dataset[ThemeDataAttribute.Light];
      delete htmlRoot.dataset[ThemeDataAttribute.Dark];
      break;
    case ThemeMode.Light:
      htmlRoot.dataset[ThemeDataAttribute.Light] = "";
      delete htmlRoot.dataset[ThemeDataAttribute.Dark];
      break;
    case ThemeMode.Dark:
      delete htmlRoot.dataset[ThemeDataAttribute.Light];
      htmlRoot.dataset[ThemeDataAttribute.Dark] = "";
      break;
  }
});
