import Tooltip from "@corvu/tooltip";
import { ComponentProps, ParentProps, splitProps } from "solid-js";

import styles from "./SidebarNavItem.module.sass";
import { UiValue } from "@constants";

export interface SidebarNavItemProps {
  /**
   * Don't add padding in the button, perfect for "background" images
   */
  fullSized?: boolean;

  tooltip?: string;

  onClick?: () => void;
}

export function SidebarNavItem(
  props: ParentProps<SidebarNavItemProps> & ComponentProps<"button" | "li">,
) {
  const [_, restProps] = splitProps(props, [
    "fullSized",
    "tooltip",
    "onClick",
    "children",
  ]);

  if (props.tooltip) {
    return (
      <Tooltip
        placement="bottom"
        openDelay={UiValue.TooltipOpenDelayMs}
        hoverableContent={false}
      >
        <Tooltip.Trigger
          as="button"
          classList={{
            [styles.nav_item]: true,
            [styles.nav_item_full]: props.fullSized,
          }}
          onClick={props.onClick}
          {...restProps as ComponentProps<"button">}
        >
          {props.children}
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content class={styles.tooltip}>
            {props.tooltip}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip>
    );
  } else {
    return (
      <li class={styles.nav_item} {...restProps as ComponentProps<"li">}>
        {props.children}
      </li>
    );
  }
}
