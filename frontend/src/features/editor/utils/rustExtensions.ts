import { autocompletion } from "@codemirror/autocomplete";
import { rust } from "@codemirror/lang-rust";
import { syntaxHighlighting } from "@codemirror/language";
import { lintGutter } from "@codemirror/lint";
import { basicSetup } from "codemirror";
import { EditorConfig } from "@constants";
import { rsgroundTheme } from "./rsgroundTheme";

export function rustExtensions(
  styles: Record<string, string>,
  includeFallbackCompletion = true,
) {
  return [
    basicSetup,
    syntaxHighlighting(rsgroundTheme),
    lintGutter(),
    ...(includeFallbackCompletion
      ? [
          autocompletion({
            interactionDelay: EditorConfig.CompletionInteractionDelayMs,
            activateOnTypingDelay: EditorConfig.CompletionActivateOnTypingDelayMs,
            closeOnBlur: false,
            tooltipClass: () => styles.completion_tooltip,
            optionClass: (completion) => {
              const typeClass = completion.type
                ? " " + styles["completion_t_" + completion.type]
                : "";

              return styles.completion_option + typeClass;
            },
          }),
        ]
      : []),
    rust(),
  ];
}
