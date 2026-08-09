// @vitest-environment jsdom

import { describe, expect, it } from "vitest";

import { ansiToHtml } from "./ansiToHtml";

describe("ansiToHtml", () => {
  it("keeps plain output as text without unnecessary spans", () => {
    const node = ansiToHtml("plain output");

    expect(node.tagName).toBe("PRE");
    expect(node.textContent).toBe("plain output");
    expect(node.querySelector("span")).toBeNull();
  });

  it("renders styles and resets them at ANSI reset sequences", () => {
    const node = ansiToHtml("before \x1b[1;31mred\x1b[0m after");
    const spans = [...node.querySelectorAll("span")];

    expect(node.textContent).toBe("before red after");
    expect(spans).toHaveLength(1);
    expect(spans[0].textContent).toBe("red");
    expect(spans[0].className).not.toBe("");
    expect(node.lastChild?.textContent).toBe(" after");
  });

  it("supports indexed and RGB extended colors", () => {
    const node = ansiToHtml(
      "\x1b[38;5;1mindexed\x1b[39m \x1b[48;2;1;2;3mRGB\x1b[49m",
    );
    const spans = [...node.querySelectorAll("span")];

    expect(spans).toHaveLength(2);
    expect(spans[0].textContent).toBe("indexed");
    expect(spans[0].style.color).toBe("rgb(221, 33, 64)");
    expect(spans[1].textContent).toBe("RGB");
    expect(spans[1].style.backgroundColor).toBe("rgb(1, 2, 3)");
  });
});
