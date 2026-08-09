// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  goToRedirectUrl,
  REDIRECT_KEY,
  saveRedirectUrl,
} from "./redirect_url";

describe("OAuth redirect storage", () => {
  beforeEach(() => {
    window.sessionStorage.clear();
    window.history.replaceState({}, "", "/");
  });

  afterEach(() => {
    window.history.replaceState({}, "", "/");
  });

  it("keeps the full same-origin location in the current tab", () => {
    window.history.replaceState({}, "", "/project?panel=output#editor");

    saveRedirectUrl();

    expect(window.sessionStorage.getItem(REDIRECT_KEY)).toBe(
      "/project?panel=output#editor",
    );
  });

  it("restores a saved location and consumes it once", () => {
    window.sessionStorage.setItem(
      REDIRECT_KEY,
      "/project?panel=output#editor",
    );
    window.history.replaceState({}, "", "/auth/callback?code=secret");

    expect(goToRedirectUrl()).toBe("/project");
    expect(window.location.pathname).toBe("/project");
    expect(window.location.search).toBe("?panel=output");
    expect(window.location.hash).toBe("#editor");
    expect(window.sessionStorage.getItem(REDIRECT_KEY)).toBeNull();
  });

  it("falls back to the root for unsafe or callback redirects", () => {
    for (const value of [
      "https://example.invalid/steal",
      "//example.invalid/steal",
      "/auth/callback?code=again",
    ]) {
      window.sessionStorage.setItem(REDIRECT_KEY, value);
      window.history.replaceState({}, "", "/auth/callback");

      expect(goToRedirectUrl()).toBe("/");
      expect(window.location.pathname).toBe("/");
    }
  });

  it("does not save the callback route as a new destination", () => {
    window.sessionStorage.setItem(REDIRECT_KEY, "/project");
    window.history.replaceState({}, "", "/auth/callback?code=secret");

    saveRedirectUrl();

    expect(window.sessionStorage.getItem(REDIRECT_KEY)).toBe("/project");
  });
});
