import { batch } from "solid-js";
import { setAuthInfo, setIsLoadingAuthInfo, goToRedirectUrl } from "../stores";
import { authCallback } from "../services";

export function interceptAuthCallback(): Promise<void> {
    if (window.location.pathname === "/auth/callback") {
        const url = new URL(window.location.href);
        const code = url.searchParams.get("code");

        if (code) {
      return handleAuthCallback(code);
    }
  }

  return Promise.resolve();
}

async function handleAuthCallback(code: string) {
  setIsLoadingAuthInfo(true);
  try {
    const authInfo = await authCallback(code);

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
