import { useState, useEffect } from "react";
import type { ConnectionProfile } from "../types/connection";
import { sshDisconnect, sshStatus } from "../lib/ipc";
import { VersionPanel } from "./panels/VersionPanel";

type Tab = "versions";

interface Props {
  profile: ConnectionProfile;
  onDisconnected: () => void;
}

const TABS: { id: Tab; label: string; icon: string }[] = [
  { id: "versions", label: "Versions", icon: "📦" },
];

export function DashboardPage({ profile, onDisconnected }: Props) {
  const [activeTab, setActiveTab] = useState<Tab>("versions");
  const [connected, setConnected] = useState<boolean | null>(null);

  // Poll SSH connection status every 5 seconds
  useEffect(() => {
    let cancelled = false;
    async function check() {
      try {
        const ok = await sshStatus(profile.id!);
        if (!cancelled) setConnected(ok);
      } catch {
        if (!cancelled) setConnected(false);
      }
    }
    check();
    const interval = setInterval(check, 5000);
    return () => { cancelled = true; clearInterval(interval); };
  }, [profile.id]);

  async function handleDisconnect() {
    await sshDisconnect(profile.id!);
    onDisconnected();
  }

  const statusColor = connected === null
    ? "bg-yellow-500 animate-pulse"
    : connected ? "bg-green-500" : "bg-red-500";
  const statusText = connected === null ? "Checking..." : connected ? "Connected" : "Disconnected";

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Sidebar */}
      <aside className="w-56 bg-gray-800 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <div className="text-green-400 font-bold text-sm">⛏️ MC Manager</div>
          <div className="text-xs text-gray-300 mt-1 font-medium truncate">{profile.name}</div>
          <div className="text-xs text-gray-500 truncate">{profile.username}@{profile.host}:{profile.port}</div>
          <div className="mt-2 flex items-center gap-1.5">
            <span className={`w-2 h-2 rounded-full ${statusColor}`} />
            <span className="text-xs text-gray-400">{statusText}</span>
          </div>
        </div>
        <nav className="flex-1 p-2 space-y-1">
          {TABS.map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`w-full text-left px-3 py-2 rounded text-sm flex items-center gap-2 transition-colors ${
                activeTab === tab.id
                  ? "bg-gray-700 text-white"
                  : "text-gray-400 hover:text-gray-200 hover:bg-gray-700/50"
              }`}
            >
              <span>{tab.icon}</span>
              {tab.label}
            </button>
          ))}
        </nav>
        <div className="p-3 border-t border-gray-700">
          <button
            onClick={handleDisconnect}
            className="w-full text-sm text-red-400 hover:text-red-300 border border-red-800 hover:border-red-600 rounded py-2"
          >
            Disconnect
          </button>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto bg-gray-900">
        <div className={activeTab === "versions" ? "" : "hidden"}><VersionPanel profileId={profile.id!} /></div>
      </main>
    </div>
  );
}
