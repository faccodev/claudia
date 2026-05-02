<div align="center">
  <img src="https://github.com/user-attachments/assets/92fd93ed-e71b-4b94-b270-50684323dd00" alt="Claudia Logo" width="120" height="120">

  <h1>Claudia</h1>

  <p>
    <strong>Powerful GUI + Web Interface for Claude Code</strong>
  </p>
  <p>
    Use Claudia as a <strong>desktop app</strong> or deploy on a <strong>VPS</strong> and access via browser from anywhere.
  </p>

  <p>
    <a href="#features"><img src="https://img.shields.io/badge/Features-✨-blue?style=for-the-badge" alt="Features"></a>
    <a href="#desktop-version"><img src="https://img.shields.io/badge/Desktop-🖥️-green?style=for-the-badge" alt="Desktop"></a>
    <a href="#server-vps-version"><img src="https://img.shields.io/badge/Server/VPS-🌐-orange?style=for-the-badge" alt="Server"></a>
    <a href="#development"><img src="https://img.shields.io/badge/Develop-🛠️-purple?style=for-the-badge" alt="Development"></a>
  </p>
</div>

![457013521-6133a738-d0cb-4d3e-8746-c6768c82672c](https://github.com/user-attachments/assets/a028de9e-d881-44d8-bae5-7326ab3558b9)

https://github.com/user-attachments/assets/bf0bdf9d-ba91-45af-9ac4-7274f57075cf

## 🌟 Two Ways to Use Claudia

| Desktop App | Server/VPS |
|------------|------------|
| Run locally on your machine | Deploy on a VPS and access via browser |
| Native desktop experience | Access from any device, anywhere |
| Perfect for local development | Share with your team or access remotely |
| All data stored locally | Data stored on server |

> [!TIP]
> **⭐ Star the repo and follow [@getAsterisk](https://x.com/getAsterisk) on X for early access to `asteria-swe-v0`**.

## 🌟 Overview

**Claudia** transforms how you interact with Claude Code - whether you prefer a native desktop app or accessing it from anywhere via web browser.

- **Desktop Version**: Built with Tauri 2 for a native, fast experience
- **Server Version**: Deploy on any VPS with nginx, SSL, and auto-start

Both versions share the same powerful features: custom agents, session management, usage analytics, MCP server management, and timeline checkpoints.

---

## 🚀 Quick Start

### Desktop (5 minutes)

```bash
git clone https://github.com/faccodev/claudia.git
cd claudia
bun install
bun run tauri dev
```

### Server/VPS (one command)

```bash
curl -sL https://raw.githubusercontent.com/faccodev/claudia/main/server/install.sh | bash -s -- \
  --domain your-domain.com \
  --api-key sk-ant-xxxxx \
  --admin-user admin \
  --admin-password yourPassword123
```

---

## ✨ Features

### 🗂️ **Project & Session Management**
- **Visual Project Browser**: Navigate through all your Claude Code projects in `~/.claude/projects/`
- **Session History**: View and resume past coding sessions with full context
- **Smart Search**: Find projects and sessions quickly with built-in search
- **Session Insights**: See first messages, timestamps, and session metadata at a glance

### 🤖 **CC Agents**
- **Custom AI Agents**: Create specialized agents with custom system prompts and behaviors
- **Agent Library**: Build a collection of purpose-built agents for different tasks
- **Background Execution**: Run agents in separate processes for non-blocking operations
- **Execution History**: Track all agent runs with detailed logs and performance metrics

### 📊 **Usage Analytics Dashboard**
- **Cost Tracking**: Monitor your Claude API usage and costs in real-time
- **Token Analytics**: Detailed breakdown by model, project, and time period
- **Visual Charts**: Beautiful charts showing usage trends and patterns
- **Export Data**: Export usage data for accounting and analysis

### 🔌 **MCP Server Management**
- **Server Registry**: Manage Model Context Protocol servers from a central UI
- **Easy Configuration**: Add servers via UI or import from existing configs
- **Connection Testing**: Verify server connectivity before use
- **Claude Desktop Import**: Import server configurations from Claude Desktop

### ⏰ **Timeline & Checkpoints**
- **Session Versioning**: Create checkpoints at any point in your coding session
- **Visual Timeline**: Navigate through your session history with a branching timeline
- **Instant Restore**: Jump back to any checkpoint with one click
- **Fork Sessions**: Create new branches from existing checkpoints
- **Diff Viewer**: See exactly what changed between checkpoints

### 📝 **CLAUDE.md Management**
- **Built-in Editor**: Edit CLAUDE.md files directly within the app
- **Live Preview**: See your markdown rendered in real-time
- **Project Scanner**: Find all CLAUDE.md files in your projects
- **Syntax Highlighting**: Full markdown support with syntax highlighting

---

## 🖥️ Desktop Version

### Prerequisites

- **Claude Code CLI**: Install from [Claude's official site](https://claude.ai/code)
- **Rust** (1.70.0 or later)
- **Bun** (latest version)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/faccodev/claudia.git
cd claudia

# Install dependencies
bun install

# Run in development mode
bun run tauri dev

# Build for production
bun run tauri build
```

### System Dependencies

**Linux (Ubuntu/Debian)**
```bash
sudo apt update && sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev
```

**macOS**
```bash
xcode-select --install
```

---

## 🌐 Server/VPS Version

Claudia Server turns any VPS into a personal Claude Code web interface. Access from any browser!

### Features

- **nginx reverse proxy** with automatic SSL (Let's Encrypt)
- **UFW/firewalld** firewall configuration
- **Systemd service** with auto-start on reboot
- **WebSocket support** for real-time agent output
- **JWT authentication** with argon2 password hashing
- **Full API** covering all Claudia features

### Installation

#### Interactive Installation

```bash
# SSH into your VPS and run:
curl -sL https://raw.githubusercontent.com/faccodev/claudia/main/server/install.sh | bash
```

The script will ask for:
- Domain name (optional)
- Anthropic API key
- Admin username and password

#### Automated Installation

```bash
curl -sL https://raw.githubusercontent.com/faccodev/claudia/main/server/install.sh | bash -s -- \
  --domain claude.example.com \
  --api-key sk-ant-api03-xxxxx \
  --admin-user admin \
  --admin-password MySecurePassword123
```

#### Full Options

| Option | Description | Default |
|--------|-------------|---------|
| `--domain` | Domain for nginx + SSL | - |
| `--port` | Server port | 3000 |
| `--api-key` | Anthropic API key | - |
| `--api-url` | Custom API URL (for proxies) | api.anthropic.com |
| `--admin-user` | Admin username | admin |
| `--admin-password` | Admin password | (random) |
| `--dir` | Installation directory | ~/claudia-server |
| `--skip-deps` | Skip dependency installation | false |
| `--skip-firewall` | Skip firewall setup | false |
| `--skip-ssl` | Skip SSL certificate | false |

### After Installation

```bash
# Check service status
sudo systemctl status claudia-server

# View logs
sudo journalctl -u claudia-server -f

# Restart service
sudo systemctl restart claudia-server

# Stop/Start
sudo systemctl stop claudia-server
sudo systemctl start claudia-server
```

### Access URLs

| Setup | URL |
|-------|-----|
| With domain + SSL | `https://your-domain.com` |
| IP only (no domain) | `http://YOUR_VPS_IP:3000` |

### Environment Variables

Located at `~/claudia-server/.env`:

```bash
CLAUDIA_PORT=3000
CLAUDIA_HOST=127.0.0.1
CLAUDIA_JWT_SECRET=your-secret-key
ANTHROPIC_API_KEY=sk-ant-xxxxx
ANTHROPIC_API_URL=https://api.anthropic.com
RUST_LOG=info
```

### Uninstallation

```bash
# Stop and disable service
sudo systemctl stop claudia-server
sudo systemctl disable claudia-server

# Remove service file
sudo rm /etc/systemd/system/claudia-server.service
sudo systemctl daemon-reload

# Remove installation directory
rm -rf ~/claudia-server

# Remove nginx config (optional)
sudo rm /etc/nginx/sites-available/claudia-server
sudo rm /etc/nginx/sites-enabled/claudia-server
sudo systemctl reload nginx
```

---

## 🛠️ Development

### Tech Stack

| Layer | Desktop | Server |
|-------|---------|--------|
| Frontend | React 18 + TypeScript + Vite | Same React (build for web) |
| Backend | Rust + Tauri 2 | Rust + Axum |
| Database | SQLite (rusqlite) | SQLite (rusqlite) |
| UI | Tailwind CSS v4 + shadcn/ui | Same |
| Auth | Tauri security model | JWT + argon2 |

### Project Structure

```
claudia/
├── src/                   # React frontend (shared)
│   ├── components/        # UI components
│   ├── lib/               # API clients (tauri + web)
│   └── lib/web-api.ts     # Web API client for server
├── src-tauri/             # Desktop backend (Tauri/Rust)
│   └── src/
│       ├── commands/      # Tauri commands
│       └── main.rs
├── server/                # Server backend (Axum/Rust)
│   ├── Cargo.toml
│   ├── install.sh          # VPS installation script
│   └── src/
│       ├── main.rs        # Axum server
│       ├── api/           # REST API handlers
│       ├── auth.rs        # JWT authentication
│       └── state.rs       # App state
└── public/                # Static assets
```

### Development Commands

```bash
# Desktop development
bun run tauri dev

# Server development
cd server
cargo run

# Frontend only (web mode)
VITE_API_URL=http://localhost:3000 bun run dev
```

---

## 🔒 Security

Claudia prioritizes your privacy and security:

1. **Process Isolation**: Agents run in separate processes
2. **Permission Control**: Configure file and network access per agent
3. **Local Storage (Desktop)**: All data stays on your machine
4. **No Telemetry**: No data collection or tracking
5. **Open Source**: Full transparency through open source code
6. **SSL/TLS**: Automatic HTTPS on server deployment
7. **JWT Auth**: Secure token-based authentication

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Areas for Contribution

- 🐛 Bug fixes and improvements
- ✨ New features and enhancements
- 📚 Documentation improvements
- 🎨 UI/UX enhancements
- 🧪 Test coverage
- 🌐 Internationalization

---

## 📄 License

This project is licensed under the AGPL License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app/) - The secure framework for building desktop apps
- Server powered by [Axum](https://github.com/tokio-rs/axum)
- [Claude](https://claude.ai) by Anthropic

---

<div align="center">
  <p>
    <strong>Made with ❤️ by the <a href="https://asterisk.so/">Asterisk</a></strong>
  </p>
  <p>
    <a href="https://github.com/faccodev/claudia/issues">Report Bug</a>
    ·
    <a href="https://github.com/faccodev/claudia/issues">Request Feature</a>
  </p>
</div>

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=faccodev/claudia&type=Date)](https://www.star-history.com/#faccodev/claudia&Date)