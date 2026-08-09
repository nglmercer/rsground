import Popover from "@corvu/popover";
import { createEffect, createSelector, createSignal, For } from "solid-js";

import { ChevronDownIcon } from "@icons/ChevronDown";
import { SelectFieldConfig } from "@constants";

import styles from "./SelectField.module.sass";

type PopoverRootProps = Parameters<typeof Popover>[0];
export type Placement = NonNullable<PopoverRootProps["placement"]>;

export type SelectFieldProps<T extends string> =
  & {
    /**
     * Posible options to select, value and showed text is the same.
     */
    options: T[];

    /**
     * Text to show when there's no option selected.
     */
    defaultText?: string;

    /**
     * Whether or not will be close once the user selects an option.
     * @defaultValue true
     */
    closeOnChange?: boolean;

    /**
     * Whether or not is editable
     */
    disabled?: boolean;

    /**
     * Position of the floating options
     */
    placement?: Placement;

    /**
     * Controlled value
     */
    value?: T;

    /**
     * Value callback
     */
    onValueChange?: (value: T) => void;
  }
  & (
    | {
      /**
       * Specific name for `inputs`, should be unique
       */
      name: string;
    }
    | {
      /**
       * Use as auto generated name with desired prefix, this will use
       * incremental and unique id as suffix. `$PREFIX-$ID`
       */
      prefix: string;
    }
    | {}
  );

let nextSelectId = 0;

export function SelectField<T extends string>(props: SelectFieldProps<T>) {
  let name: string;

  if ("name" in props) {
    name = props.name;
  } else if ("prefix" in props) {
    name = props.prefix + "-" + nextSelectId++;
  } else {
    name = SelectFieldConfig.DefaultPrefix + nextSelectId++;
  }

  const [open, setOpen] = createSignal(false);

  const [selected, setSelected] = createSignal(
    props.defaultText ? null : props.options[0],
  );

  const selectedSelector = createSelector(selected);

  createEffect(() => {
    if (props.value != null) {
      setSelected(() => props.value);
    }
  });

  createEffect(() => {
    if (props.disabled) setOpen(false);
  });

  return (
    <Popover
      open={open()}
      onOpenChange={(open) => !props.disabled && setOpen(open)}
      placement={props.placement ?? SelectFieldConfig.DefaultPlacement}
    >
      <Popover.Trigger
        classList={{ [styles.base]: true, [styles.disabled]: props.disabled }}
      >
        <span>{selected() ?? props.defaultText}</span>
        <div>
          <ChevronDownIcon width="0.5em" height="0.5em" />
        </div>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content as="ul" class={styles.options}>
          <For each={props.options}>
            {(item) => (
              <label class={styles.item}>
                <input
                  type="radio"
                  name={name}
                  checked={selectedSelector(item)}
                  onChange={() => {
                    setSelected(() => item);
                    props.onValueChange?.(item);

                    if (props.closeOnChange ?? true) {
                      setOpen(false);
                    }
                  }}
                />
                <span>{item}</span>
              </label>
            )}
          </For>
        </Popover.Content>
      </Popover.Portal>
    </Popover>
  );
}
