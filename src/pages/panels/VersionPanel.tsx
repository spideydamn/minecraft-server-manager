import { useState, useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { listMcVersions, installServerVersion, getServerConfig } from "../../lib/ipc";

interface Props {
  profileId: number;
}
interface Version { id: string; type: string; releaseTime: string; }

// Matches the LogLine struct emitted by the Rust backend
interface LogLine {
  raw: string;
  level: "Info" | "Warn" | "Error" | "Other";
  timestamp: string | null;
}

export function VersionPanel({ profileId }: Props) {
  const [allVersions, setAllVersions] = useState<Version[]>([]);
  const [versions, setVersions] = useState<Version[]>([]);
  const [snapshots, setSnapshots] = useState(false);
  const [selected, setSelected] = useState<string>("");
  const [installed, setInstalled] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Install progress state
  const [installing, setInstalling] = useState(false);
  const [installLogs, setInstallLogs] = useState<LogLine[]>([]);
  const [installDone, setInstallDone] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  const logEndRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    loadVersions();
    loadConfig();
    // Cleanup listener on unmount
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [profileId]);

  // Filter versions based on snapshots checkbox
  useEffect(() => {
    if (snapshots) {
      setVersions(allVersions);
    } else {
      setVersions(allVersions.filter(v => v.type === "release"));
    }
  }, [snapshots, allVersions]);

  // Auto-scroll log drawer on new lines
  useEffect(() => {
    if (logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [installLogs]);


  async function loadVersions() {
    try {
      // Always fetch all versions (including snapshots) and filter locally
      const data = await listMcVersions(true);
      setAllVersions(data);
      // Initial filter based on snapshots state
      if (snapshots) {
        setVersions(data);
      } else {
        setVersions(data.filter(v => v.type === "release"));
      }
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function loadConfig() {
    try {
      const cfg = await getServerConfig(profileId);
      setInstalled(cfg.minecraft_version ?? null);
    } catch {}
  }

  async function handleInstall() {
    if (!selected) return;

    // Subscribe to install_log events BEFORE starting install to avoid race condition
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }

    // Reset log drawer state and start install
    setInstallLogs([]);
    setInstallDone(false);
    setInstallError(null);
    setError(null);
    setInstalling(true);

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<LogLine>("install_log", (event) => {
        setInstallLogs(prev => [...prev, event.payload]);
      });
      unlistenRef.current = unlisten;

      await installServerVersion(profileId, selected);
      setInstalled(selected);
      setInstallDone(true);
    } catch (e: any) {
      setInstallError(e.toString());
    } finally {
      setInstalling(false);
      // Keep listener alive a bit longer to catch final events, then remove
      setTimeout(() => {
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }
      }, 500);
    }
  }

  function dismissLogs() {
    setInstallLogs([]);
    setInstallDone(false);
    setInstallError(null);
  }

  function forceReset() {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    setInstalling(false);
    setInstallLogs([]);
    setInstallDone(false);
    setInstallError(null);
  }

  function logLineColor(level: LogLine["level"]): string {
    switch (level) {
      case "Error": return "text-red-400";
      case "Warn":  return "text-yellow-400";
      case "Info":  return "text-gray-300";
      default:      return "text-gray-400";
    }
  }

  const showDrawer = installing || installDone || installError !== null;

  return (
    <div className="p-8">
      <h2 className="text-2xl font-bold text-gray-100 mb-6">Minecraft Version</h2>

      {error && <div className="bg-red-900/40 border border-red-700 text-red-300 rounded p-3 mb-4">{error}</div>}

      <div className="bg-gray-800 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-semibold text-gray-200">Available Versions</h3>
          <div className="flex items-center gap-2 text-sm text-gray-400">
            <input
              type="checkbox"
              id="show-snapshots"
              checked={snapshots}
              onChange={e => setSnapshots(e.target.checked)}
              className="cursor-pointer"
            />
            <label htmlFor="show-snapshots" className="cursor-pointer">Show snapshots</label>
          </div>
        </div>

        <div className="mb-4">
          <select
            className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-gray-200"
            value={selected}
            onChange={e => setSelected(e.target.value)}
            disabled={installing}
          >
            <option value="">Select a version...</option>
            {versions.map(v => (
              <option key={v.id} value={v.id}>{v.id} ({v.type}) — {new Date(v.releaseTime).toLocaleDateString()}</option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={handleInstall}
            disabled={!selected || installing}
            className="bg-green-600 hover:bg-green-500 disabled:opacity-40 text-white px-6 py-2 rounded"
          >
            {installing ? "Installing..." : installed === selected ? "Reinstall" : "Install Server"}
          </button>
          {installing && (
            <button
              onClick={forceReset}
              className="text-xs text-red-400 hover:text-red-200 px-3 py-2 border border-red-700 rounded"
              title="Force reset stuck installation state"
            >
              Force Reset
            </button>
          )}
        </div>
      </div>

      {/* Install log drawer */}
      {showDrawer && (
        <div className="mt-6 bg-gray-900 border border-gray-700 rounded-lg overflow-hidden">
          <div className="flex items-center justify-between px-4 py-2 bg-gray-800 border-b border-gray-700">
            <span className="text-sm font-semibold text-gray-300">Installation Log</span>
            {installing ? (
              <button
                onClick={forceReset}
                className="text-xs text-red-400 hover:text-red-200 px-3 py-1 border border-red-700 rounded"
                title="Force reset stuck installation state"
              >
                Force Reset
              </button>
            ) : (
              <button
                onClick={dismissLogs}
                className="text-xs text-gray-400 hover:text-gray-200 px-3 py-1 border border-gray-600 rounded"
              >
                {installDone ? "OK" : "Dismiss"}
              </button>
            )}
          </div>

          <div className="p-3 h-48 overflow-y-auto font-mono text-xs space-y-0.5">
            {installLogs.map((line, i) => (
              <div key={i} className={logLineColor(line.level)}>
                {line.timestamp && <span className="text-gray-500 mr-2">[{line.timestamp}]</span>}
                {line.raw}
              </div>
            ))}
            {installing && (
              <div className="text-gray-500 flex items-center gap-2">
                <span className="animate-spin inline-block">⟳</span>
                <span>Installing...</span>
              </div>
            )}
            <div ref={logEndRef} />
          </div>

          {installDone && (
            <div className="px-4 py-2 bg-green-900/40 border-t border-green-800 text-green-300 text-sm font-medium">
              ✓ Installation complete
            </div>
          )}
          {installError && (
            <div className="px-4 py-2 bg-red-900/40 border-t border-red-800 text-red-300 text-sm">
              ✗ Installation failed: {installError}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
