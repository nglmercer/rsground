import Popover, { DynamicProps } from "@corvu/popover";
import { For, JSX, ParentProps, splitProps, ValidComponent } from "solid-js";
import { Dynamic } from "solid-js/web";
import { ContextMenuConfig, ContextMenuLevel } from "@constants";

import {
  addContextMenu,
  closeAllContextMenus,
  contextMenus,
  openContextMenu,
  setContextMenu,
} from "../stores";

import styles from "./ContextMenu.module.sass";

type ContextMenuItem = {
  level?: ContextMenuLevel;
  disabled?: boolean | (() => boolean);
  onClick?: () => void;
};

export interface ContextMenuProps {
  options: Record<
    string,
    ContextMenuItem | JSX.Element
  >;

  /**
   * Open dialog when user clicks with left click
   * @defaultvalue false
   */
  useLeftClick?: boolean;

  /**
   * Open dialog when user clicks with right click
   * @defaultvalue true
   */
  useRightClick?: boolean;

  /**
   * Spawn dialog on cursor position
   * @defaultValue true
   */
  followCursor?: boolean;

  /** Called immediately before the menu is opened. */
  onOpen?: (event: MouseEvent) => void;
}

function isDisabled(item: ContextMenuItem) {
  return typeof item.disabled === "function"
    ? item.disabled()
    : item.disabled;
}

export function ContextMenu(
  props_: DynamicProps<
    ValidComponent,
    ParentProps<ContextMenuProps>
  >,
) {
  const [props, restProps] = splitProps(props_, [
    "as",
    "children",
    "options",
    "useLeftClick",
    "useRightClick",
    "followCursor",
    "onOpen",
  ]);

  const contextMenuId = addContextMenu();
  let anchorRef!: HTMLElement;

  const openOnMouseEvent = (ev: MouseEvent) => {
    ev.preventDefault();
    ev.stopPropagation();

    closeAllContextMenus();

    props.onOpen?.(ev);

    // Align context menu arrow with cursor event
    if (props.followCursor != false) {
      anchorRef.style.top = `${ev.clientY - ContextMenuConfig.CursorAnchorOffsetPx}px`;
      anchorRef.style.left = `${ev.clientX + ContextMenuConfig.CursorLeftOffsetPx}px`;
    }
    openContextMenu(contextMenuId);
  };

  return (
    <Popover
      open={contextMenus[contextMenuId]}
      onOpenChange={setContextMenu.bind(null, contextMenuId)}
      placement={ContextMenuConfig.Placement}
      closeOnEscapeKeyDown
      closeOnOutsidePointer
      closeOnOutsideFocus={false}
      trapFocus={true}
    >
      <Dynamic
        {...restProps}
        component={props.as}
        onClick={(ev: MouseEvent) => {
          restProps.onClick?.(ev);
          props.useLeftClick == true && openOnMouseEvent(ev);
        }}
        onContextMenu={props.useRightClick != false && openOnMouseEvent}
      >
        <Popover.Anchor
          class={styles.anchor}
          ref={(r) => anchorRef = r}
        />

        {props.children}
      </Dynamic>

      <Popover.Portal>
        <Popover.Content
          as="ul"
          class={styles.content}
        >
          <For each={Object.entries(props.options)}>
            {([name, item]) => (
              // This excludes JSX element from object defined item
              typeof item !== "object" || item instanceof Array ||
                item instanceof Node
                ? item
                : (
                  <li
                    tabindex="1"
                    classList={{
                      [styles.disabled]: isDisabled(item),

                      [styles.item]: ![
                        ContextMenuLevel.Error,
                        ContextMenuLevel.Warning,
                      ].includes(item.level),
                      [styles.item_error]: item.level === ContextMenuLevel.Error,
                      [styles.item_warning]: item.level === ContextMenuLevel.Warning,
                    }}
                    onClick={() => {
                      if (!isDisabled(item)) item.onClick?.();
                    }}
                  >
                    {name}
                  </li>
                )
            )}
          </For>
        </Popover.Content>
      </Popover.Portal>
    </Popover>
  );
}
