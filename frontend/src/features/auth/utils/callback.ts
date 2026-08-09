import { batch } from "solid-js";
import { setAuthInfo, setIsLoadingAuthInfo, goToRedirectUrl } from "../stores";
import { authCallback } from "../services";

export function interceptAuthCallback(): Promise<void> {
    if (window.location.pathname === "/auth/callback") {
        const url = new URL(window.location.href);
        const code = url.searchParams.get("code");
        const state = url.searchParams.get("state");

        if (code && state) {
      return handleAuthCallback(code, state);
    }
  }

  return Promise.resolve();
}

async function handleAuthCallback(code: string, state: string) {
  setIsLoadingAuthInfo(true);
  try {
    const authInfo = await authCallback(code, state);

    batch(() => {
      setAuthInfo(authInfo);
      setIsLoadingAuthInfo(false);
    });

    goToRedirectUrl();
  } catch (error) {
    setIsLoadingAuthInfo(false);
    throw error;
  }
}
