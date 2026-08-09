import { Route, StorageKey } from "@constants";

export const REDIRECT_KEY = StorageKey.RedirectUrl;

function redirectStorage(): Storage {
  try {
    // Redirects are temporary and should stay in the tab that started OAuth.
    return window.sessionStorage;
  } catch {
    // Storage can be disabled by privacy settings. Keep the flow usable where
    // possible rather than failing before the browser can leave for GitHub.
    return window.localStorage;
  }
}

function isSafeRedirect(value: string | null): value is string {
  if (!value || !value.startsWith("/") || value.startsWith("//")) return false;

  try {
    const target = new URL(value, window.location.origin);
    return target.origin === window.location.origin &&
      target.pathname !== Route.AuthCallback;
  } catch {
    return false;
  }
}

/**
 * Replace the OAuth callback URL with the saved same-origin destination.
 * Keeping this as a history replacement avoids an extra page load and keeps
 * the callback code and state out of the browser history.
 */
export function goToRedirectUrl(): string {
  const storage = redirectStorage();
  const savedUrl = storage.getItem(REDIRECT_KEY);
  storage.removeItem(REDIRECT_KEY);

  const target = isSafeRedirect(savedUrl) ? savedUrl : Route.Root;
  const url = new URL(target, window.location.origin);

  window.history.replaceState(
    {},
    "",
    `${url.pathname}${url.search}${url.hash}`,
  );

  return url.pathname;
}

/**
 * Save the current same-origin location for the OAuth callback.
 */
export function saveRedirectUrl() {
  const url = new URL(window.location.href);

  // Never save the callback itself. Doing so would make a failed or repeated
  // OAuth flow redirect back into the callback route.
  if (url.pathname === Route.AuthCallback) return;

  redirectStorage().setItem(
    REDIRECT_KEY,
    `${url.pathname}${url.search}${url.hash}`,
  );
}

/**
 * Leave the callback route in a clean, recoverable state after a malformed or
 * failed callback. The app can then show an error with a retry action without
 * trying to process the same query parameters forever.
 */
export function clearAuthCallbackUrl() {
  const url = new URL(window.location.href);
  url.pathname = Route.Root;
  url.search = "";
  url.hash = "";

  window.history.replaceState({}, "", url);
}
