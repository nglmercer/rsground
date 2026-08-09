// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from "vitest";

const { authCallback } = vi.hoisted(() => ({
  authCallback: vi.fn(),
}));

vi.mock("../services", () => ({ authCallback }));
vi.mock("../stores", async () => {
  const redirect = await import("../stores/redirect_url");

  return {
    ...redirect,
    setAuthInfo: vi.fn(),
    setIsLoadingAuthInfo: vi.fn(),
  };
});

import { interceptAuthCallback } from "./callback";
import { REDIRECT_KEY } from "../stores/redirect_url";

describe("interceptAuthCallback", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
    window.history.replaceState({}, "", "/");
  });

  it("rejects an incomplete callback and leaves a clean root URL", async () => {
    window.history.replaceState({}, "", "/auth/callback?code=only-code");

    await expect(interceptAuthCallback()).rejects.toThrow(
      "valid sign-in response",
    );
    expect(window.location.pathname).toBe("/");
    expect(window.location.search).toBe("");
    expect(authCallback).not.toHaveBeenCalled();
  });

  it("stores the returned user and restores the requested project", async () => {
    authCallback.mockResolvedValue({
      jwt: "jwt",
      id: "octocat",
      is_guest: false,
      name: "octocat",
    });
    window.sessionStorage.setItem(REDIRECT_KEY, "/project?panel=code");
    window.history.replaceState(
      {},
      "",
      "/auth/callback?code=code&state=state",
    );

    await expect(interceptAuthCallback()).resolves.toBeUndefined();

    expect(authCallback).toHaveBeenCalledWith("code", "state");
    expect(window.location.pathname).toBe("/project");
    expect(window.location.search).toBe("?panel=code");
  });
});
