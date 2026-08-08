import styles from "./ansiToHtml.module.sass";

const ANSI_CLASS_BY_CODE: Record<number, string | undefined> = {
  1: styles.mods_bold,
  3: styles.mods_italic,
  4: styles.mods_underline,
  30: styles.fore_black,
  31: styles.fore_red,
  32: styles.fore_green,
  33: styles.fore_yellow,
  34: styles.fore_blue,
  35: styles.fore_magenta,
  36: styles.fore_cyan,
  37: styles.fore_white,
  40: styles.back_black,
  41: styles.back_red,
  42: styles.back_green,
  43: styles.back_yellow,
  44: styles.back_blue,
  45: styles.back_magenta,
  46: styles.back_cyan,
  47: styles.back_white,
  90: styles.fore_bright_black,
  91: styles.fore_bright_red,
  92: styles.fore_bright_green,
  93: styles.fore_bright_yellow,
  94: styles.fore_bright_blue,
  95: styles.fore_bright_magenta,
  96: styles.fore_bright_cyan,
  97: styles.fore_bright_white,
  100: styles.back_bright_black,
  101: styles.back_bright_red,
  102: styles.back_bright_green,
  103: styles.back_bright_yellow,
  104: styles.back_bright_blue,
  105: styles.back_bright_magenta,
  106: styles.back_bright_cyan,
  107: styles.back_bright_white,
};

type AnsiState = {
  classes: Set<string>;
  foreground?: string;
  background?: string;
};

const ANSI_SEQUENCE = /\x1b\[([0-9;]*)m/g;

export function ansiToHtml(text: string): HTMLElement {
  const node = document.createElement("pre");
  node.className = styles.mods_reset;

  const state: AnsiState = { classes: new Set() };
  let lastIndex = 0;

  for (const match of text.matchAll(ANSI_SEQUENCE)) {
    appendText(node, text.slice(lastIndex, match.index), state);
    applyCodes(match[1], state);
    lastIndex = (match.index ?? 0) + match[0].length;
  }

  appendText(node, text.slice(lastIndex), state);
  return node;
}

function appendText(node: HTMLElement, text: string | undefined, state: AnsiState) {
  if (!text) return;

  const hasFormatting =
    state.classes.size > 0 || state.foreground !== undefined ||
    state.background !== undefined;

  if (!hasFormatting) {
    node.append(text);
    return;
  }

  const span = document.createElement("span");
  state.classes.forEach((className) => span.classList.add(className));
  if (state.foreground) span.style.color = state.foreground;
  if (state.background) span.style.backgroundColor = state.background;
  span.textContent = text;
  node.append(span);
}

function applyCodes(rawCodes: string, state: AnsiState) {
  const codes = rawCodes === "" ? [0] : rawCodes.split(";").map(Number);

  for (let index = 0; index < codes.length; index++) {
    const code = codes[index];

    if (code === 0) {
      state.classes.clear();
      state.foreground = undefined;
      state.background = undefined;
      continue;
    }

    if (code === 22) state.classes.delete(styles.mods_bold);
    else if (code === 23) state.classes.delete(styles.mods_italic);
    else if (code === 24) state.classes.delete(styles.mods_underline);
    else if (code === 39) state.foreground = undefined;
    else if (code === 49) state.background = undefined;
    else if (code === 38 || code === 48) {
      const color = readExtendedColor(codes, index + 1);
      if (color) {
        if (code === 38) state.foreground = color.value;
        else state.background = color.value;
        index = color.lastIndex;
      }
    } else {
      const className = ANSI_CLASS_BY_CODE[code];
      if (className) {
        if (code >= 30 && code <= 37 || code >= 90 && code <= 97) {
          removeColorClasses(state, false);
        } else if (code >= 40 && code <= 47 || code >= 100 && code <= 107) {
          removeColorClasses(state, true);
        }
        state.classes.add(className);
      }
    }
  }
}

function readExtendedColor(
  codes: number[],
  modeIndex: number,
): { value: string; lastIndex: number } | undefined {
  const mode = codes[modeIndex];
  if (mode === 5 && codes[modeIndex + 1] !== undefined) {
    return {
      value: colorFromIndex(codes[modeIndex + 1]),
      lastIndex: modeIndex + 1,
    };
  }

  if (
    mode === 2 &&
    codes[modeIndex + 1] !== undefined &&
    codes[modeIndex + 2] !== undefined &&
    codes[modeIndex + 3] !== undefined
  ) {
    return {
      value: `rgb(${codes[modeIndex + 1]}, ${codes[modeIndex + 2]}, ${codes[modeIndex + 3]})`,
      lastIndex: modeIndex + 3,
    };
  }
}

function removeColorClasses(state: AnsiState, background: boolean) {
  Object.entries(ANSI_CLASS_BY_CODE).forEach(([rawCode, className]) => {
    const code = Number(rawCode);
    const isBackground = code >= 40 && code <= 47 || code >= 100 && code <= 107;
    if (className && isBackground === background) state.classes.delete(className);
  });
}

function colorFromIndex(index: number): string {
  const baseColors = [
    [0, 0, 0], [221, 33, 64], [0, 255, 0], [255, 255, 0],
    [0, 68, 170], [255, 0, 255], [0, 255, 255], [255, 255, 255],
  ];

  if (index < 8) return rgb(baseColors[index] ?? baseColors[0]);
  if (index < 16) return rgb(baseColors[index - 8] ?? baseColors[0]);
  if (index < 232) {
    const value = [0, 95, 135, 175, 215, 255];
    const cubeIndex = index - 16;
    return rgb([
      value[Math.floor(cubeIndex / 36)],
      value[Math.floor(cubeIndex / 6) % 6],
      value[cubeIndex % 6],
    ]);
  }

  const gray = 8 + (index - 232) * 10;
  return `rgb(${gray}, ${gray}, ${gray})`;
}

function rgb(values: number[]) {
  return `rgb(${values[0]}, ${values[1]}, ${values[2]})`;
}
