import { invoke } from "@tauri-apps/api/core";
import type { ConnectionProfile } from "../types/connection";

// --- Connection Profiles ---
export const listProfiles = () => invoke<ConnectionProfile[]>("list_profiles");
export const createProfile = (p: Omit<ConnectionProfile, "id">) =>
  invoke<number>("create_profile", { ...p, authMethod: p.authMethod });
export const updateProfile = (p: ConnectionProfile) =>
  invoke<void>("update_profile", { ...p });
export const deleteProfile = (id: number) =>
  invoke<void>("delete_profile", { id });

// --- SSH ---
export const sshConnect = (profileId: number, p: ConnectionProfile) =>
  invoke<void>("ssh_connect", {
    profileId,
    host: p.host,
    port: p.port,
    username: p.username,
    authMethod: p.authMethod,
    password: p.password,
    keyPath: p.keyPath,
  });
export const sshDisconnect = (profileId: number) =>
  invoke<void>("ssh_disconnect", { profileId });
export const sshStatus = (profileId: number) =>
  invoke<boolean>("ssh_status", { profileId });

// --- Version management ---
export const listMcVersions = (includeSnapshots: boolean) =>
  invoke<{ id: string; type: string; releaseTime: string }[]>("list_mc_versions", { includeSnapshots });
export const installServerVersion = (profileId: number, versionId: string) =>
  invoke<string>("install_server_version", { profileId, versionId });

// --- Settings ---
export const getServerConfig = (profileId: number) =>
  invoke<any>("get_server_config", { profileId });
