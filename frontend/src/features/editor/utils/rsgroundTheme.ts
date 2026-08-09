import { HighlightStyle } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

import styles from "./rsgroundTheme.module.sass";

export const rsgroundTheme = HighlightStyle.define([
  { tag: t.keyword, class: styles.keyword },
  {
    tag: [
      t.function(t.name),
      t.function(t.propertyName),
      t.labelName,
      t.macroName,
    ],
    class: styles.fn,
  },
  { tag: [t.typeName, t.namespace], class: styles.struct },
  { tag: t.string, class: styles.string },
  { tag: t.meta, class: styles.attribute },
  { tag: t.special(t.variableName), class: styles.lifetime },
  { tag: t.variableName, class: styles.variable },
  { tag: t.constant(t.variableName), class: styles.constant },
  { tag: t.number, class: styles.number },
  { tag: t.bool, class: styles.bool },
  { tag: t.comment, class: styles.comment },
  { tag: t.operator, class: styles.operator },
]);
