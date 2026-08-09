import { AccessLevel } from "@features/ws/types";

export interface ProjectInfo {
  id: string,
  name?: string,
  owner?: string,
  users: Record<string, [string, AccessLevel]> | null,
  requests: Record<string, string>,
  is_public: boolean,
  has_password?: boolean,
  is_owner?: boolean,
}
