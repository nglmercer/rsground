import { setAuthInfo, authInfo } from "../stores";
import { fetchMe, loginGithub, loginGuest } from "../services";
import { generateRandomName } from "./random_name";

export type AuthCheckResult = "ready" | "redirecting";

export async function checkForAuth(): Promise<AuthCheckResult> {
  const storedAuth = authInfo();
  const { jwt, name, is_guest } = storedAuth ?? {};

  if (!jwt) {
    await loginGuest(generateRandomName());
    return "ready";
  }

  const userInfo = await fetchMe(jwt);

  if (userInfo != null) {
    // Refresh mutable profile data (for example a GitHub avatar) from the
    // server instead of continuing with stale local storage.
    setAuthInfo(userInfo);
    return "ready";
  }

  if (is_guest) {
    await loginGuest(name);
    return "ready";
  } else {
    loginGithub();
    return "redirecting";
  }
}
