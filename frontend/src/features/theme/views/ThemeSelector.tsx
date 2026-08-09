import { createSelector, ParentProps } from "solid-js";

import { DisplayIcon } from "@icons/Display";
import { MoonIcon } from "@icons/Moon";
import { SunIcon } from "@icons/Sun";

import { setThemeMode, themeMode } from "../stores";
import { ThemeMode } from "../types";
import { ThemeSelectorConfig } from "@constants";

import styles from "./ThemeSelector.module.sass";

export function ThemeSelector() {
  const themeSelector = createSelector(themeMode);

  return (
    <li class={styles.container}>
      <ThemeSelectorItem themeSelector={themeSelector} mode={ThemeMode.System}>
        <DisplayIcon />
      </ThemeSelectorItem>
      <ThemeSelectorItem themeSelector={themeSelector} mode={ThemeMode.Light}>
        <SunIcon />
      </ThemeSelectorItem>
      <ThemeSelectorItem themeSelector={themeSelector} mode={ThemeMode.Dark}>
        <MoonIcon />
      </ThemeSelectorItem>
    </li>
  );
}

interface ThemeSelectorItemProps {
  themeSelector: (key: ThemeMode) => boolean;
  mode: ThemeMode;
}

function ThemeSelectorItem(props: ParentProps<ThemeSelectorItemProps>) {
  return (
    <label class={styles.item}>
      <input
        type="radio"
        name={ThemeSelectorConfig.InputName}
        checked={props.themeSelector(props.mode)}
        onChange={(ev) => {
          if (ev.currentTarget.checked) {
            setThemeMode(props.mode);
          }
        }}
      />
      {props.children}
    </label>
  );
}
