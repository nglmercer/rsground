import { createSignal, observable } from "solid-js";
import { AuthInfo } from "../types";
import { createLocalStoredSignal } from "@utils/createLocalStoredSignal";
import { StorageKey } from "@constants";

export const AUTH_KEY = StorageKey.Auth;

export const [authInfo, setAuthInfo] = createLocalStoredSignal<AuthInfo | null>(
  AUTH_KEY,
  null,
  (v) => JSON.parse(v),
  (v) => JSON.stringify(v)
);

export const isGithubLogged = () => !!authInfo()?.avatar_url;

export const [isLoadingAuthInfo, setIsLoadingAuthInfo] = createSignal(false);

// Sync auth info in local storage
observable(authInfo).subscribe((authInfo) => {
  if (authInfo) {
    window.localStorage.setItem(AUTH_KEY, JSON.stringify(authInfo));
  } else {
    window.localStorage.removeItem(AUTH_KEY);
  }
});
