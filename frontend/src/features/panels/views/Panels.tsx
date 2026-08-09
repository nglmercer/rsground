import { render } from "solid-js/web";
import {
  DockviewComponent,
  DockviewTheme,
  IContentRenderer,
} from "dockview-core";

import { setDockview } from "../stores";
import { flushPendingFiles } from "@features/editor/services";
import { CodePanel } from "./CodePanel";
import { OutputPanel } from "./OutputPanel";
import { DockviewConfig, Panel } from "@constants";

import "dockview-core/dist/styles/dockview.css";
import "./dockview.sass";

import styles from "./Panels.module.sass";

export function Panels() {
  const element = <div class={styles.container} /> as HTMLElement;

  const dockview = new DockviewComponent(element, {
    theme: {
      name: DockviewConfig.ThemeName,
      className: DockviewConfig.ThemeClassName,
      gap: DockviewConfig.GapPx,
      dndOverlayMounting: DockviewConfig.OverlayMounting,
      dndPanelOverlay: DockviewConfig.OverlayGroup,
    } satisfies DockviewTheme,
    disableFloatingGroups: true,
    singleTabMode: DockviewConfig.SingleTabMode,

    createComponent(options) {
      const element = document.createElement("div");
      const dispose = render(
        () =>
          options.name == Panel.Code ? (
            <CodePanel id={options.id} />
          ) : options.name == Panel.Output ? (
            <OutputPanel />
          ) : (
            <span>Esto es canallesco</span>
          ),
        element,
      );

      return {
        element,
        init(_params) {},
        dispose,
      } satisfies IContentRenderer;
    },
  });

  dockview.api.onDidRemovePanel((e) => {
    if (e.id == Panel.Output) {
      dockview.api.addPanel({
        id: Panel.Output,
        component: Panel.Output,
        title: Panel.OutputTitle,
        initialHeight: DockviewConfig.OutputInitialHeight,
        minimumHeight: DockviewConfig.OutputMinimumHeight,
        position: { direction: DockviewConfig.OutputDirection },
      });
    }
  });

  setDockview(dockview.api);

  dockview.api.addPanel({
    id: Panel.Output,
    component: Panel.Output,
    title: Panel.OutputTitle,
    initialHeight: DockviewConfig.OutputInitialHeight,
    minimumHeight: DockviewConfig.OutputMinimumHeight,
    position: { direction: DockviewConfig.OutputDirection },
  });

  flushPendingFiles();

  return element;
}
