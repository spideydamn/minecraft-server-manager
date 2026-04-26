# ⛏️ Minecraft Server Manager

A cross-platform desktop application to manage a Minecraft Java Edition server on a remote Linux VM via SSH.

Built with [Tauri](https://tauri.app/) (Rust + React + TypeScript).

## Current Status

This is an early-stage development project. Currently implemented features:

- 🔒 SSH connection management (profiles with host, port, username, password/key)
- 📦 Minecraft version fetching and display
- 💾 Local SQLite database for storing connection profiles and settings
- 🎨 Basic UI with Connection and Dashboard pages

## Prerequisites

### On your desktop (developer)
- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS

### On the remote VM
- Linux (Ubuntu/Debian recommended)
- Java 17+ (`sudo apt-get install openjdk-21-jdk-headless`)
- SSH server running and accessible

## Getting Started

```bash
# Clone the repository
git clone https://github.com/spideydamn/minecraft-server-manager.git
cd minecraft-server-manager

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Architecture

- **Frontend**: React + TypeScript + Tailwind CSS (Vite)
- **Backend**: Rust (Tauri) with `russh` for SSH, `rusqlite` for local storage
- **Database**: SQLite for persistent storage of profiles and settings
