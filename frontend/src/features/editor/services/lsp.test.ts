// @vitest-environment jsdom

import { describe, expect, it } from "vitest";

import {
  fileUri,
  pathFromUri,
  sanitizeLspHtml,
} from "../utils/lspUtils";

describe("Rust Analyzer URI and documentation boundaries", () => {
  it("round-trips nested and encoded project paths", () => {
    const path = "src/hello world-你好.rs";
    expect(pathFromUri(fileUri(path))).toBe(path);
    expect(fileUri(path)).toBe(
      "file:///home/src/hello%20world-%E4%BD%A0%E5%A5%BD.rs",
    );
  });

  it("rejects paths outside the project home", () => {
    expect(pathFromUri("file:///etc/passwd")).toBeNull();
    expect(pathFromUri("file:///home/../secret.rs")).toBeNull();
    expect(pathFromUri("file:///home/src/%2E%2E/secret.rs")).toBeNull();
    expect(pathFromUri("file:///home/src/%2Fetc%2Fpasswd")).toBeNull();
  });

  it("sanitizes Rust Analyzer markdown before rendering it", () => {
    const sanitized = sanitizeLspHtml(
      '<script>alert(1)</script><a href="javascript:alert(2)" onclick="alert(3)" style="background:url(javascript:alert(4))">unsafe</a><a href="https://example.test">safe</a>',
    );

    expect(sanitized).not.toContain("script");
    expect(sanitized).not.toContain("javascript:");
    expect(sanitized).not.toContain("onclick");
    expect(sanitized).not.toContain("style");
    expect(sanitized).toContain('href="https://example.test"');
  });
});
