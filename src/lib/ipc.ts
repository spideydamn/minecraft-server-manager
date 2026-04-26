import { invoke } from "@tauri-apps/api/core";
import type { ConnectionProfile } from "../types/connection";

// --- Connection Profiles ---
export const listProfiles = () => invoke<ConnectionProfile[]>("list_profiles");
export const createProfile = (p: Omit<ConnectionProfile, "id">) =>
  invoke<number>("create_profile", { profile: { ...p, authMethod: p.authMethod } });
export const updateProfile = (p: ConnectionProfile) =>
  invoke<void>("update_profile", { id: p.id, profile: { ...p, authMethod: p.authMethod } });
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
export const listInstalledVersions = (profileId: number) =>
  invoke<{ versionId: string; jarName: string; serverDir: string; inUse: boolean; installationDate: string }[]>("list_installed_versions", { profileId });
export const deleteVersion = (profileId: number, versionId: string) =>
  invoke<string>("delete_version", { profileId, versionId });
export const reinstallVersion = (profileId: number, versionId: string) =>
  invoke<string>("reinstall_version", { profileId, versionId });

// --- Settings ---
export const getServerConfig = (profileId: number) =>
  invoke<any>("get_server_config", { profileId });
