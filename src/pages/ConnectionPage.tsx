import { useState, useEffect } from "react";
import type { ConnectionProfile } from "../types/connection";
import { listProfiles, createProfile, updateProfile, deleteProfile, sshConnect } from "../lib/ipc";

interface Props {
  onConnected: (profile: ConnectionProfile) => void;
}

export function ConnectionPage({ onConnected }: Props) {
  const [profiles, setProfiles] = useState<ConnectionProfile[]>([]);
  const [editing, setEditing] = useState<ConnectionProfile | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [connecting, setConnecting] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadProfiles();
  }, []);

  async function loadProfiles() {
    try {
      const data = await listProfiles();
      setProfiles(data);
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function handleConnect(profile: ConnectionProfile) {
    setConnecting(profile.id!);
    setError(null);
    try {
      await sshConnect(profile.id!, profile);
      onConnected(profile);
    } catch (e: any) {
      setError(`Connection failed: ${e}`);
    } finally {
      setConnecting(null);
    }
  }

  async function handleSave(profile: ConnectionProfile) {
    if (profile.id) {
      await updateProfile(profile);
    } else {
      await createProfile(profile);
    }
    setShowForm(false);
    setEditing(null);
    loadProfiles();
  }

  async function handleDelete(id: number) {
    if (confirm("Delete this profile?")) {
      await deleteProfile(id);
      loadProfiles();
    }
  }

  return (
    <div className="flex flex-col items-center justify-center min-h-screen p-8">
      <div className="w-full max-w-2xl">
        <h1 className="text-3xl font-bold text-green-400 mb-2">⛏️ Minecraft Server Manager</h1>
        <p className="text-gray-400 mb-8">Connect to your remote VM to manage your server</p>

        {error && (
          <div className="bg-red-900/50 border border-red-500 rounded p-3 mb-4 text-red-300">
            {error}
          </div>
        )}

        <div className="flex justify-between items-center mb-4">
          <h2 className="text-lg font-semibold text-gray-200">Connection Profiles</h2>
          <button
            onClick={() => { setEditing(null); setShowForm(true); }}
            className="bg-green-600 hover:bg-green-500 text-white px-4 py-2 rounded text-sm"
          >
            + New Profile
          </button>
        </div>

        {profiles.length === 0 && !showForm && (
          <div className="text-center text-gray-500 py-12 border border-dashed border-gray-700 rounded">
            No profiles yet. Create one to get started.
          </div>
        )}

        <div className="space-y-3">
          {profiles.map(p => (
            <div key={p.id} className="bg-gray-800 rounded p-4 flex items-center justify-between">
              <div>
                <div className="font-medium text-gray-100">{p.name}</div>
                <div className="text-sm text-gray-400">{p.username}@{p.host}:{p.port} ({p.authMethod})</div>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => { setEditing(p); setShowForm(true); }}
                  className="text-gray-400 hover:text-gray-200 text-sm px-3 py-1 border border-gray-600 rounded"
                >
                  Edit
                </button>
                <button
                  onClick={() => handleDelete(p.id!)}
                  className="text-red-400 hover:text-red-300 text-sm px-3 py-1 border border-red-800 rounded"
                >
                  Delete
                </button>
                <button
                  onClick={() => handleConnect(p)}
                  disabled={connecting === p.id}
                  className="bg-green-600 hover:bg-green-500 disabled:opacity-50 text-white text-sm px-4 py-1 rounded"
                >
                  {connecting === p.id ? "Connecting..." : "Connect"}
                </button>
              </div>
            </div>
          ))}
        </div>

        {showForm && (
          <ProfileForm
            initial={editing}
            onSave={handleSave}
            onCancel={() => { setShowForm(false); setEditing(null); }}
          />
        )}
      </div>
    </div>
  );
}

interface FormProps {
  initial: ConnectionProfile | null;
  onSave: (p: ConnectionProfile) => void;
  onCancel: () => void;
}

function ProfileForm({ initial, onSave, onCancel }: FormProps) {
  const [form, setForm] = useState<ConnectionProfile>(initial ?? {
    name: "", host: "", port: 22, username: "", authMethod: "password",
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  function validate() {
    const e: Record<string, string> = {};
    if (!form.name.trim()) e.name = "Name is required";
    if (!form.host.trim()) e.host = "Host is required";
    if (!form.username.trim()) e.username = "Username is required";
    return e;
  }

  function handleSubmit() {
    const e = validate();
    if (Object.keys(e).length > 0) { setErrors(e); return; }
    onSave(form);
  }

  return (
    <div className="mt-6 bg-gray-800 rounded p-6 border border-gray-700">
      <h3 className="text-lg font-semibold mb-4">{initial ? "Edit Profile" : "New Profile"}</h3>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Profile Name" error={errors.name}>
          <input className="input" value={form.name} onChange={e => setForm({...form, name: e.target.value})} />
        </Field>
        <Field label="Host / IP" error={errors.host}>
          <input className="input" value={form.host} onChange={e => setForm({...form, host: e.target.value})} />
        </Field>
        <Field label="Port">
          <input className="input" type="number" value={form.port} onChange={e => setForm({...form, port: +e.target.value})} />
        </Field>
        <Field label="Username" error={errors.username}>
          <input className="input" value={form.username} autoCapitalize="none" autoCorrect="off" spellCheck={false} onChange={e => setForm({...form, username: e.target.value})} />
        </Field>
        <Field label="Auth Method">
          <select className="input" value={form.authMethod} onChange={e => setForm({...form, authMethod: e.target.value as any})}>
            <option value="password">Password</option>
            <option value="key">SSH Key</option>
          </select>
        </Field>
        {form.authMethod === "password" ? (
          <Field label="Password">
            <input className="input" type="password" value={form.password ?? ""} onChange={e => setForm({...form, password: e.target.value})} />
          </Field>
        ) : (
          <Field label="Key Path">
            <input className="input" value={form.keyPath ?? ""} placeholder="/home/user/.ssh/id_rsa" onChange={e => setForm({...form, keyPath: e.target.value})} />
          </Field>
        )}
      </div>
      <div className="flex gap-3 mt-6">
        <button onClick={handleSubmit} className="bg-green-600 hover:bg-green-500 text-white px-6 py-2 rounded">Save</button>
        <button onClick={onCancel} className="text-gray-400 hover:text-gray-200 px-6 py-2 border border-gray-600 rounded">Cancel</button>
      </div>
    </div>
  );
}

function Field({ label, children, error }: { label: string; children: React.ReactNode; error?: string }) {
  return (
    <div>
      <label className="block text-sm text-gray-400 mb-1">{label}</label>
      {children}
      {error && <p className="text-red-400 text-xs mt-1">{error}</p>}
    </div>
  );
}
