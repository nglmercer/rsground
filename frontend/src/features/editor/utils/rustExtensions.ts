import { autocompletion, CompletionContext } from "@codemirror/autocomplete";
import { rust, rustLanguage } from "@codemirror/lang-rust";
import { syntaxHighlighting } from "@codemirror/language";
import { basicSetup } from "codemirror";
import { EditorConfig } from "@constants";
import { rsgroundTheme } from "./rsgroundTheme";

const keywords = [
  "pub",
  "fn",
  "struct",
  "let",
];

const localCompletions = [
  ...(keywords.map((keyword) => ({
    label: keyword,
      type: EditorConfig.CompletionKeywordType,
  }))),
];

export function rustExtensions(styles: Record<string, string>) {
  return [
    basicSetup,
    syntaxHighlighting(rsgroundTheme),
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
    rust(),
    rustLanguage.data.of({
      "autocomplete": async (context: CompletionContext) => {
        let word = context.matchBefore(/\w*/);
        if (word.from == word.to && !context.explicit) {
          return null;
        }

        return {
          from: word.from,
          options: localCompletions,
        };
      },
    }),
  ];
}
