export interface AuthInfo {
  jwt: string;
  id: string;
  is_guest: boolean;
  name: string;

  avatar_url?: string;
}

/** Payload returned by `/auth/me`; the JWT stays in the local session. */
export interface AuthVerification {
  id: string;
  name: string;
  is_guest: boolean;
  exp: number;
}
