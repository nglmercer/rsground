import styles from "./ansiToHtml.module.sass";
import { AnsiCode, AnsiPalette } from "@constants";

const ANSI_CLASS_BY_CODE: Record<number, string | undefined> = {
  [AnsiCode.Bold]: styles.mods_bold,
  [AnsiCode.Italic]: styles.mods_italic,
  [AnsiCode.Underline]: styles.mods_underline,
  [AnsiCode.ForegroundBlack]: styles.fore_black,
  [AnsiCode.ForegroundBlack + 1]: styles.fore_red,
  [AnsiCode.ForegroundBlack + 2]: styles.fore_green,
  [AnsiCode.ForegroundBlack + 3]: styles.fore_yellow,
  [AnsiCode.ForegroundBlack + 4]: styles.fore_blue,
  [AnsiCode.ForegroundBlack + 5]: styles.fore_magenta,
  [AnsiCode.ForegroundBlack + 6]: styles.fore_cyan,
  [AnsiCode.ForegroundWhite]: styles.fore_white,
  [AnsiCode.BackgroundBlack]: styles.back_black,
  [AnsiCode.BackgroundBlack + 1]: styles.back_red,
  [AnsiCode.BackgroundBlack + 2]: styles.back_green,
  [AnsiCode.BackgroundBlack + 3]: styles.back_yellow,
  [AnsiCode.BackgroundBlack + 4]: styles.back_blue,
  [AnsiCode.BackgroundBlack + 5]: styles.back_magenta,
  [AnsiCode.BackgroundBlack + 6]: styles.back_cyan,
  [AnsiCode.BackgroundWhite]: styles.back_white,
  [AnsiCode.ForegroundBrightBlack]: styles.fore_bright_black,
  [AnsiCode.ForegroundBrightBlack + 1]: styles.fore_bright_red,
  [AnsiCode.ForegroundBrightBlack + 2]: styles.fore_bright_green,
  [AnsiCode.ForegroundBrightBlack + 3]: styles.fore_bright_yellow,
  [AnsiCode.ForegroundBrightBlack + 4]: styles.fore_bright_blue,
  [AnsiCode.ForegroundBrightBlack + 5]: styles.fore_bright_magenta,
  [AnsiCode.ForegroundBrightBlack + 6]: styles.fore_bright_cyan,
  [AnsiCode.ForegroundBrightWhite]: styles.fore_bright_white,
  [AnsiCode.BackgroundBrightBlack]: styles.back_bright_black,
  [AnsiCode.BackgroundBrightBlack + 1]: styles.back_bright_red,
  [AnsiCode.BackgroundBrightBlack + 2]: styles.back_bright_green,
  [AnsiCode.BackgroundBrightBlack + 3]: styles.back_bright_yellow,
  [AnsiCode.BackgroundBrightBlack + 4]: styles.back_bright_blue,
  [AnsiCode.BackgroundBrightBlack + 5]: styles.back_bright_magenta,
  [AnsiCode.BackgroundBrightBlack + 6]: styles.back_bright_cyan,
  [AnsiCode.BackgroundBrightWhite]: styles.back_bright_white,
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
  const codes = rawCodes === "" ? [AnsiCode.Reset] : rawCodes.split(";").map(Number);

  for (let index = 0; index < codes.length; index++) {
    const code = codes[index];

    if (code === AnsiCode.Reset) {
      state.classes.clear();
      state.foreground = undefined;
      state.background = undefined;
      continue;
    }

    if (code === AnsiCode.BoldOff) state.classes.delete(styles.mods_bold);
    else if (code === AnsiCode.ItalicOff) state.classes.delete(styles.mods_italic);
    else if (code === AnsiCode.UnderlineOff) state.classes.delete(styles.mods_underline);
    else if (code === AnsiCode.ForegroundReset) state.foreground = undefined;
    else if (code === AnsiCode.BackgroundReset) state.background = undefined;
    else if (code === AnsiCode.ForegroundExtended || code === AnsiCode.BackgroundExtended) {
      const color = readExtendedColor(codes, index + 1);
      if (color) {
        if (code === AnsiCode.ForegroundExtended) state.foreground = color.value;
        else state.background = color.value;
        index = color.lastIndex;
      }
    } else {
      const className = ANSI_CLASS_BY_CODE[code];
      if (className) {
        if (
          code >= AnsiCode.ForegroundBlack &&
          code <= AnsiCode.ForegroundWhite ||
          code >= AnsiCode.ForegroundBrightBlack &&
          code <= AnsiCode.ForegroundBrightWhite
        ) {
          removeColorClasses(state, false);
        } else if (
          code >= AnsiCode.BackgroundBlack &&
          code <= AnsiCode.BackgroundWhite ||
          code >= AnsiCode.BackgroundBrightBlack &&
          code <= AnsiCode.BackgroundBrightWhite
        ) {
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
  if (mode === AnsiCode.IndexedColorMode && codes[modeIndex + 1] !== undefined) {
    return {
      value: colorFromIndex(codes[modeIndex + 1]),
      lastIndex: modeIndex + 1,
    };
  }

  if (
    mode === AnsiCode.RgbColorMode &&
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
    const isBackground =
      code >= AnsiCode.BackgroundBlack &&
      code <= AnsiCode.BackgroundWhite ||
      code >= AnsiCode.BackgroundBrightBlack &&
      code <= AnsiCode.BackgroundBrightWhite;
    if (className && isBackground === background) state.classes.delete(className);
  });
}

function colorFromIndex(index: number): string {
  const baseColors = AnsiPalette.Base;

  if (index < baseColors.length) return rgb(baseColors[index] ?? baseColors[0]);
  if (index < baseColors.length * 2) {
    return rgb(baseColors[index - baseColors.length] ?? baseColors[0]);
  }
  if (index < AnsiCode.ColorCubeEnd) {
    const cubeIndex = index - AnsiCode.ColorCubeStart;
    return rgb([
      AnsiPalette.ColorCube[Math.floor(cubeIndex / AnsiCode.ColorCubeRedDivisor)],
      AnsiPalette.ColorCube[
        Math.floor(cubeIndex / AnsiCode.ColorCubeGreenDivisor) % AnsiCode.ColorCubeSize
      ],
      AnsiPalette.ColorCube[cubeIndex % AnsiCode.ColorCubeSize],
    ]);
  }

  const gray =
    AnsiCode.GrayscaleBase +
    (index - AnsiCode.GrayscaleStart) * AnsiCode.GrayscaleStep;
  return `rgb(${gray}, ${gray}, ${gray})`;
}

function rgb(values: readonly number[]) {
  return `rgb(${values[0]}, ${values[1]}, ${values[2]})`;
}
