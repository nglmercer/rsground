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
      <span class={styles.heading}>Theme</span>
      <div class={styles.options} role="radiogroup" aria-label="Theme mode">
        <ThemeSelectorItem
          themeSelector={themeSelector}
          mode={ThemeMode.System}
          label="System"
        >
          <DisplayIcon aria-hidden="true" />
        </ThemeSelectorItem>
        <ThemeSelectorItem
          themeSelector={themeSelector}
          mode={ThemeMode.Light}
          label="Light"
        >
          <SunIcon aria-hidden="true" />
        </ThemeSelectorItem>
        <ThemeSelectorItem
          themeSelector={themeSelector}
          mode={ThemeMode.Dark}
          label="Dark"
        >
          <MoonIcon aria-hidden="true" />
        </ThemeSelectorItem>
      </div>
    </li>
  );
}

interface ThemeSelectorItemProps {
  themeSelector: (key: ThemeMode) => boolean;
  mode: ThemeMode;
  label: string;
}

function ThemeSelectorItem(props: ParentProps<ThemeSelectorItemProps>) {
  return (
    <label class={styles.item} title={props.label}>
      <input
        type="radio"
        name={ThemeSelectorConfig.InputName}
        aria-label={props.label}
        checked={props.themeSelector(props.mode)}
        onChange={(ev) => {
          if (ev.currentTarget.checked) {
            setThemeMode(props.mode);
          }
        }}
      />
      <span class={styles.icon} aria-hidden="true">{props.children}</span>
      <span>{props.label}</span>
    </label>
  );
}
