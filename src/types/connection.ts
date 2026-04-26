export type AuthMethod = "password" | "key";

export interface ConnectionProfile {
  id?: number;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  password?: string;
  keyPath?: string;
}

export type ConnectionStatus = "connected" | "disconnected" | "connecting";
