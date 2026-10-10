export type AppCategory =
  | "image"
  | "vector"
  | "video"
  | "audio"
  | "pdf"
  | "text"
  | "layout"
  | "data"
  | "cad"
  | "system"
  | "other";

export type AppStatus = "stable" | "beta" | "alpha" | "planned";

export type LaunchKind = "server" | "url" | "command";

export interface AppLaunchSpec {
  kind: LaunchKind;
  /** Executable to run. Defaults to the current node binary. */
  command?: string;
  args?: string[];
  url?: string;
  /** Preferred port. Absent/0 = the launcher picks a free one. */
  port?: number;
  /** Open a dedicated app window. Default true. */
  window?: boolean;
  /** Path polled until the app answers. Default "/". */
  readyPath?: string;
}

export interface AppManifest {
  id: string;
  name: string;
  version: string;
  coreApiVersion: string;
  description?: string;
  author?: string;
  category?: AppCategory;
  status?: AppStatus;
  inspiredBy?: string;
  icon?: string;
  entry?: string;
  protocolEntry?: string;
  mcpEntry?: string;
  launch?: AppLaunchSpec;
  protocolRange?: string;
  permissions?: string[];
  capabilities?: string[];
  minShell?: string;
  updateChannel?: "stable" | "beta";
  integrity?: string;
  signature?: string;
}
