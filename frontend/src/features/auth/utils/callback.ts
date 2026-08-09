import { batch } from "solid-js";
import {
  clearAuthCallbackUrl,
  goToRedirectUrl,
  setAuthInfo,
  setIsLoadingAuthInfo,
} from "../stores";
import { authCallback } from "../services";
import { QueryParam, Route } from "@constants";

export async function interceptAuthCallback(): Promise<void> {
  if (window.location.pathname !== Route.AuthCallback) return;

  const url = new URL(window.location.href);
  const code = url.searchParams.get(QueryParam.Code);
  const state = url.searchParams.get(QueryParam.State);

  if (!code || !state) {
    clearAuthCallbackUrl();
    throw new Error("GitHub did not return a valid sign-in response.");
  }

  try {
    await handleAuthCallback(code, state);
  } catch (error) {
    clearAuthCallbackUrl();
    throw error instanceof Error
      ? error
      : new Error("GitHub sign-in failed. Please try again.");
  }
}

async function handleAuthCallback(code: string, state: string) {
  setIsLoadingAuthInfo(true);

  try {
    const authInfo = await authCallback(code, state);

    batch(() => {
      setAuthInfo(authInfo);
    });

    goToRedirectUrl();
  } catch (error) {
    throw error instanceof Error
      ? error
      : new Error("GitHub sign-in failed. Please try again.");
  } finally {
    setIsLoadingAuthInfo(false);
  }
}
