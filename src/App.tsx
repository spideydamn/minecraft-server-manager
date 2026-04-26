import { useState } from "react";
import { ConnectionPage } from "./pages/ConnectionPage";
import { DashboardPage } from "./pages/DashboardPage";
import type { ConnectionProfile } from "./types/connection";

type Page = "connection" | "dashboard";

function App() {
  const [page, setPage] = useState<Page>("connection");
  const [activeProfile, setActiveProfile] = useState<ConnectionProfile | null>(null);

  const handleConnected = (profile: ConnectionProfile) => {
    setActiveProfile(profile);
    setPage("dashboard");
  };

  const handleDisconnected = () => {
    setActiveProfile(null);
    setPage("connection");
  };

  return (
    <div className="min-h-screen bg-gray-900 text-gray-100">
      {page === "connection" && (
        <ConnectionPage onConnected={handleConnected} />
      )}
      {page === "dashboard" && activeProfile && (
        <DashboardPage
          profile={activeProfile}
          onDisconnected={handleDisconnected}
        />
      )}
    </div>
  );
}

export default App;
