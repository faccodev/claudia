#!/bin/bash
# Claudia Server - Complete Installation Script
# ============================================
# One-command installer for Claudia Server on VPS
# Handles everything: dependencies, build, domain, SSL, firewall, auto-start

set -e  # Exit on error

# ═══════════════════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════════════════════════

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

BOLD='\033[1m'

# Default settings
DEFAULT_PORT=3000
DEFAULT_HOST="0.0.0.0"
REPO_URL="https://github.com/faccodev/claudia.git"
LOG_FILE="/tmp/claudia-install-$(date +%Y%m%d-%H%M%S).log"

# ═══════════════════════════════════════════════════════════════════════════════════════
# Logging Functions
# ═══════════════════════════════════════════════════════════════════════════════════════

log() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}[✓]${NC} $1" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[✗ ERROR]${NC} $1" | tee -a "$LOG_FILE"
    exit 1
}

section() {
    echo
    echo -e "${CYAN}${BOLD}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}${BOLD}  $1${NC}"
    echo -e "${CYAN}${BOLD}═══════════════════════════════════════════════════════════${NC}"
    echo
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Environment Detection
# ═══════════════════════════════════════════════════════════════════════════════════════

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        VER=$VERSION_ID
        OS_NAME="$NAME"
    else
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        VER=$(uname -r)
        OS_NAME="$OS"
    fi

    log "Detected OS: $OS_NAME $VER"

    case $OS in
        ubuntu|debian|raspbian|linuxmint)
            PKG_MANAGER="apt-get"
            PACKAGES_UPDATER="apt-get update && apt-get upgrade -y"
            PACKAGES_INSTALLER="apt-get install -y"
            ;;
        fedora|rhel|centos|almalinux|rocky)
            PKG_MANAGER="dnf"
            PACKAGES_UPDATER="dnf update -y"
            PACKAGES_INSTALLER="dnf install -y"
            ;;
        arch|manjaro)
            PKG_MANAGER="pacman"
            PACKAGES_UPDATER="pacman -Syu --noconfirm"
            PACKAGES_INSTALLER="pacman -S --noconfirm"
            ;;
        alpine)
            PKG_MANAGER="apk"
            PACKAGES_UPDATER="apk update"
            PACKAGES_INSTALLER="apk add --no-cache"
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac

    success "Package manager: $PKG_MANAGER"
}

check_root() {
    if [ "$EUID" -eq 0 ]; then
        SUDO=""
        SUDO_E=""
    else
        SUDO="sudo"
        SUDO_E="sudo"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# System Dependencies
# ═══════════════════════════════════════════════════════════════════════════════════════

install_system_deps() {
    section "Installing System Dependencies"

    log "Updating package lists..."
    eval "$PACKAGES_UPDATER" 2>&1 | tee -a "$LOG_FILE" || true

    log "Installing dependencies..."
    case $PKG_MANAGER in
        apt-get)
            $SUDO_E $PACKAGES_INSTALLER \
                build-essential \
                pkg-config \
                libssl-dev \
                curl \
                wget \
                git \
                file \
                htop \
                ufw \
                nginx \
                certbot \
                python3-certbot-nginx \
                2>&1 | tee -a "$LOG_FILE"
            ;;
        dnf)
            $SUDO_E $PACKAGES_INSTALLER \
                gcc \
                gcc-c++ \
                make \
                pkg-config \
                openssl-devel \
                curl \
                wget \
                git \
                htop \
                firewalld \
                nginx \
                certbot \
                python3-certbot-nginx \
                2>&1 | tee -a "$LOG_FILE"
            ;;
        pacman)
            $SUDO_E $PACKAGES_INSTALLER \
                base-devel \
                pkg-config \
                openssl \
                curl \
                wget \
                git \
                htop \
                ufw \
                nginx \
                certbot \
                2>&1 | tee -a "$LOG_FILE"
            ;;
        apk)
            $SUDO_E $PACKAGES_INSTALLER \
                build-base \
                pkgconfig \
                openssl-dev \
                curl \
                wget \
                git \
                htop \
                nginx \
                certbot \
                2>&1 | tee -a "$LOG_FILE"
            ;;
    esac

    success "System dependencies installed"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Rust Installation
# ═══════════════════════════════════════════════════════════════════════════════════════

install_rust() {
    section "Installing Rust"

    if command -v rustc &> /dev/null; then
        success "Rust already installed: $(rustc --version 2>&1 | head -n1)"
        return
    fi

    log "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        2>&1 | tee -a "$LOG_FILE" || error "Failed to install Rust"

    # Source cargo environment
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    success "Rust installed: $(rustc --version 2>&1 | head -n1)"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Node.js Installation
# ═══════════════════════════════════════════════════════════════════════════════════════

install_nodejs() {
    section "Installing Node.js"

    if command -v node &> /dev/null; then
        success "Node.js already installed: $(node --version)"
        return
    fi

    log "Installing Node.js..."

    # Try package manager first
    case $PKG_MANAGER in
        apt-get)
            curl -fsSL https://deb.nodesource.com/setup_lts.x | $SUDO_E bash - \
                2>&1 | tee -a "$LOG_FILE" || true
            $SUDO_E $PACKAGES_INSTALLER nodejs 2>&1 | tee -a "$LOG_FILE"
            ;;
        dnf)
            curl -fsSL https://rpm.nodesource.com/setup_lts.x | bash - \
                2>&1 | tee -a "$LOG_FILE" || true
            $SUDO_E $PACKAGES_INSTALLER nodejs 2>&1 | tee -a "$LOG_FILE"
            ;;
        pacman)
            $SUDO_E $PACKAGES_INSTALLER nodejs npm 2>&1 | tee -a "$LOG_FILE"
            ;;
        apk)
            $SUDO_E $PACKAGES_INSTALLER nodejs npm 2>&1 | tee -a "$LOG_FILE"
            ;;
    esac

    # Fallback to nvm if not installed
    if ! command -v node &> /dev/null; then
        log "Installing Node.js via NVM..."
        export NVM_DIR="$HOME/.nvm"
        curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash - \
            2>&1 | tee -a "$LOG_FILE" || true
        [ -s "$NVM_DIR/nvm.sh" ] && source "$NVM_DIR/nvm.sh"
        nvm install --lts 2>&1 | tee -a "$LOG_FILE" || true
    fi

    if command -v node &> /dev/null; then
        success "Node.js installed: $(node --version)"
    else
        warn "Node.js installation may have failed"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Claude CLI Installation
# ═══════════════════════════════════════════════════════════════════════════════════════

install_claude() {
    section "Installing Claude CLI"

    if command -v claude &> /dev/null; then
        local version
        version=$(claude --version 2>&1 | head -n1 || echo "unknown")
        success "Claude CLI already installed: $version"
        return
    fi

    log "Installing Claude CLI via npm..."

    if command -v npm &> /dev/null; then
        npm install -g @anthropic-ai/claude-code 2>&1 | tee -a "$LOG_FILE" \
            || warn "npm install failed, trying alternative method"
    fi

    if ! command -v claude &> /dev/null; then
        log "Trying alternative installation..."
        # Try via corepack
        if command -v corepack &> /dev/null; then
            corepack enable && corepack prepare npm@latest --activate 2>&1 | tee -a "$LOG_FILE" || true
        fi

        # Direct npm install
        npm install -g @anthropic-ai/claude-code --prefer-online 2>&1 | tee -a "$LOG_FILE" \
            || warn "Claude CLI installation failed. Please install manually."
    fi

    if command -v claude &> /dev/null; then
        success "Claude CLI installed: $(claude --version 2>&1 | head -n1)"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Repository Setup
# ═══════════════════════════════════════════════════════════════════════════════════════

clone_or_update_repo() {
    section "Setting Up Repository"

    if [ -d "$INSTALL_DIR/.git" ]; then
        log "Repository already exists at $INSTALL_DIR"
        read -p "Update from remote? [Y/n]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            cd "$INSTALL_DIR"
            git pull origin main 2>&1 | tee -a "$LOG_FILE"
            success "Repository updated"
        fi
    else
        log "Cloning repository from $REPO_URL..."
        git clone "$REPO_URL" "$INSTALL_DIR" 2>&1 | tee -a "$LOG_FILE" \
            || error "Failed to clone repository"
        success "Repository cloned to $INSTALL_DIR"
    fi

    cd "$INSTALL_DIR"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Build Server
# ═══════════════════════════════════════════════════════════════════════════════════════

build_server() {
    section "Building Claudia Server"

    # Source cargo environment
    export PATH="$HOME/.cargo/bin:$PATH"

    cd "$INSTALL_DIR/server"

    log "Checking Rust toolchain..."
    rustc --version 2>&1 | tee -a "$LOG_FILE"
    cargo --version 2>&1 | tee -a "$LOG_FILE"

    log "Building server (this may take a while)..."
    cargo build --release 2>&1 | tee -a "$LOG_FILE" \
        || error "Failed to build server"

    if [ ! -f "$INSTALL_DIR/server/target/release/claudia-server" ]; then
        error "Server binary not found after build"
    fi

    success "Server built successfully"
    log "Binary: $INSTALL_DIR/server/target/release/claudia-server"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Build Frontend
# ═══════════════════════════════════════════════════════════════════════════════════════

build_frontend() {
    section "Building Frontend"

    cd "$INSTALL_DIR"

    if ! command -v npm &> /dev/null; then
        warn "npm not available, skipping frontend build"
        return
    fi

    log "Installing Node dependencies..."
    npm install 2>&1 | tee -a "$LOG_FILE" || warn "npm install had issues"

    log "Building frontend..."
    VITE_API_URL="http://localhost:$PORT" npm run build 2>&1 | tee -a "$LOG_FILE" \
        || warn "Frontend build had issues"

    if [ -d "$INSTALL_DIR/dist" ]; then
        success "Frontend built"
    else
        warn "Frontend dist directory not found"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Firewall Configuration
# ═══════════════════════════════════════════════════════════════════════════════════════

configure_firewall() {
    section "Configuring Firewall"

    if command -v ufw &> /dev/null; then
        log "Configuring UFW firewall..."

        # Enable UFW
        $SUDO_E ufw --force enable 2>&1 | tee -a "$LOG_FILE"

        # Allow SSH (important to not lock yourself out!)
        $SUDO_E ufw allow 22/tcp comment 'SSH' 2>&1 | tee -a "$LOG_FILE"

        # Allow HTTP and HTTPS
        $SUDO_E ufw allow 80/tcp comment 'HTTP' 2>&1 | tee -a "$LOG_FILE"
        $SUDO_E ufw allow 443/tcp comment 'HTTPS' 2>&1 | tee -a "$LOG_FILE"

        # Allow custom port if different
        if [ "$PORT" != "80" ] && [ "$PORT" != "443" ]; then
            $SUDO_E ufw allow "$PORT/tcp" comment "Claudia Server" 2>&1 | tee -a "$LOG_FILE"
        fi

        $SUDO_E ufw reload 2>&1 | tee -a "$LOG_FILE"

        success "UFW firewall configured"
        $SUDO_E ufw status numbered 2>&1 | tee -a "$LOG_FILE"

    elif command -v firewall-cmd &> /dev/null; then
        log "Configuring firewalld..."

        $SUDO_E firewall-cmd --permanent --add-service=http 2>&1 | tee -a "$LOG_FILE"
        $SUDO_E firewall-cmd --permanent --add-service=https 2>&1 | tee -a "$LOG_FILE"
        $SUDO_E firewall-cmd --permanent --add-port="${PORT}/tcp" 2>&1 | tee -a "$LOG_FILE"
        $SUDO_E firewall-cmd --reload 2>&1 | tee -a "$LOG_FILE"

        success "firewalld configured"

    else
        warn "No firewall tool found (ufw/firewalld). Please configure manually."
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Domain and SSL Configuration
# ═══════════════════════════════════════════════════════════════════════════════════════

configure_domain_ssl() {
    section "Configuring Domain and SSL"

    if [ -z "$DOMAIN" ]; then
        echo "Enter your domain name (e.g., claudia.example.com)"
        echo "Leave empty to skip domain/SSL setup."
        echo
        read -p "Domain name: " DOMAIN
    fi

    if [ -z "$DOMAIN" ]; then
        warn "Skipping domain configuration"
        return
    fi

    log "Configuring domain: $DOMAIN"

    # Check if DNS is pointing to this server
    log "Note: Make sure $DOMAIN points to $(curl -s ifconfig.me 2>/dev/null || hostname -I | awk '{print $1}')"

    # Install nginx if needed
    if ! command -v nginx &> /dev/null; then
        log "Installing nginx..."
        $SUDO_E $PACKAGES_INSTALLER nginx 2>&1 | tee -a "$LOG_FILE"
    fi

    # Create nginx configuration
    log "Creating nginx reverse proxy configuration..."
    NGINX_CONF="/etc/nginx/sites-available/claudia-server"
    NGINX_ENABLED="/etc/nginx/sites-enabled/claudia-server"

    cat > /tmp/claudia-nginx.conf << 'NGINXCONF'
server {
    listen 80;
    server_name DOMAIN_PLACEHOLDER;

    client_max_body_size 100M;

    location / {
        proxy_pass http://127.0.0.1:PORT_PLACEHOLDER;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }

    location /ws {
        proxy_pass http://127.0.0.1:PORT_PLACEHOLDER;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_read_timeout 86400;
    }

    location /api {
        proxy_pass http://127.0.0.1:PORT_PLACEHOLDER;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
NGINXCONF

    # Replace placeholders
    sed -e "s/DOMAIN_PLACEHOLDER/$DOMAIN/g" \
        -e "s/PORT_PLACEHOLDER/$PORT/g" \
        /tmp/claudia-nginx.conf > "$NGINX_CONF"

    # Enable site
    $SUDO_E ln -sf "$NGINX_CONF" "$NGINX_ENABLED" 2>/dev/null || true

    # Remove default site
    if [ -f /etc/nginx/sites-enabled/default ]; then
        $SUDO_E rm -f /etc/nginx/sites-enabled/default
    fi

    # Test and reload nginx
    if $SUDO_E nginx -t 2>&1 | tee -a "$LOG_FILE"; then
        $SUDO_E systemctl enable nginx 2>&1 | tee -a "$LOG_FILE"
        $SUDO_E systemctl restart nginx 2>&1 | tee -a "$LOG_FILE"
        success "Nginx configured and started"
    else
        error "Nginx configuration test failed"
    fi

    # SSL Certificate
    echo
    read -p "Request SSL certificate from Let's Encrypt? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if command -v certbot &> /dev/null; then
            log "Requesting SSL certificate..."
            $SUDO_E certbot --nginx -d "$DOMAIN" --noninteractive --agree-tos \
                --email "admin@$DOMAIN" --redirect 2>&1 | tee -a "$LOG_FILE" \
                || warn "SSL certificate request failed"

            # Auto-renewal
            $SUDO_E systemctl enable certbot.timer 2>&1 | tee -a "$LOG_FILE"
            $SUDO_E systemctl start certbot.timer 2>&1 | tee -a "$LOG_FILE"
            success "SSL certificate installed and auto-renewal enabled"
        else
            warn "Certbot not installed. Run manually: certbot --nginx -d $DOMAIN"
        fi
    fi

    success "Domain configuration complete"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Systemd Service
# ═══════════════════════════════════════════════════════════════════════════════════════

create_systemd_service() {
    section "Creating Systemd Service"

    log "Creating systemd service for auto-start..."

    $SUDO_E tee /etc/systemd/system/claudia-server.service > /dev/null << EOF
[Unit]
Description=Claudia Server - Web UI for Claude Code
After=network.target
Wants=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR/server
Environment="RUST_LOG=info"
Environment="CLAUDIA_PORT=$PORT"
Environment="CLAUDIA_HOST=127.0.0.1"
ExecStart=$INSTALL_DIR/server/target/release/claudia-server
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_DIR $HOME/.claude $HOME/.local/share/claudia

[Install]
WantedBy=multi-user.target
EOF

    $SUDO_E systemctl daemon-reload 2>&1 | tee -a "$LOG_FILE"
    $SUDO_E systemctl enable claudia-server 2>&1 | tee -a "$LOG_FILE"

    success "Systemd service created and enabled"
    log "Service name: claudia-server"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Claude CLI Configuration
# ═══════════════════════════════════════════════════════════════════════════════════════

configure_claude_cli() {
    section "Configuring Claude CLI"

    echo "To use Claude Code, you need an Anthropic API key."
    echo "Get one at: https://console.anthropic.com/settings/keys"
    echo

    read -p "Anthropic API Key [leave empty to configure later]: " API_KEY
    read -p "Custom API URL (leave empty for default Anthropic): " CUSTOM_API_URL

    # Configure Claude
    mkdir -p "$HOME/.claude"
    mkdir -p "$HOME/.config/claude"

    cat > "$HOME/.config/claude/config.json" << EOF
{
    "api_key": "${API_KEY:-}",
    "api_url": "${CUSTOM_API_URL:-}",
    "max_tokens": 8192,
    "temperature": 1,
    "output_format": "stream-json"
}
EOF

    if [ -n "$API_KEY" ]; then
        export ANTHROPIC_API_KEY="$API_KEY"
    fi

    success "Claude CLI configured"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Start Services
# ═══════════════════════════════════════════════════════════════════════════════════════

start_services() {
    section "Starting Services"

    log "Starting Claudia Server..."

    if systemctl is-active --quiet claudia-server 2>/dev/null; then
        $SUDO_E systemctl restart claudia-server 2>&1 | tee -a "$LOG_FILE"
    else
        $SUDO_E systemctl start claudia-server 2>&1 | tee -a "$LOG_FILE"
    fi

    sleep 2

    if systemctl is-active --quiet claudia-server 2>/dev/null; then
        success "Claudia Server started successfully"
    else
        warn "Claudia Server may not have started. Check logs with: journalctl -u claudia-server -n 50"
    fi

    # Show service status
    echo
    $SUDO_E systemctl status claudia-server --no-pager 2>&1 | head -10 || true
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Create Environment File
# ═══════════════════════════════════════════════════════════════════════════════════════

create_env_file() {
    log "Creating environment configuration..."

    cat > "$INSTALL_DIR/.env" << EOF
# Claudia Server Configuration
# Generated by install script

# Server
CLAUDIA_PORT=$PORT
CLAUDIA_HOST=127.0.0.1
CLAUDIA_DIR=$HOME/.claude
CLAUDIA_DATA_DIR=$HOME/.local/share/claudia

# Security
CLAUDIA_JWT_SECRET=$(openssl rand -base64 32)

# Claude CLI
ANTHROPIC_API_KEY=${API_KEY:-}
ANTHROPIC_API_URL=${CUSTOM_API_URL:-https://api.anthropic.com}

# Logging
RUST_LOG=info
EOF

    success "Environment file created"
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Print Summary
# ═══════════════════════════════════════════════════════════════════════════════════════

print_summary() {
    section "Installation Complete!"

    echo -e "${GREEN}${BOLD}Claudia Server has been installed successfully!${NC}"
    echo
    echo -e "${BOLD}Configuration:${NC}"
    echo "  Installation directory: $INSTALL_DIR"
    echo "  Server port: $PORT"
    echo "  Service: claudia-server"
    echo

    if [ -n "$DOMAIN" ]; then
        echo -e "${BOLD}Access URLs:${NC}"
        echo -e "  HTTP:  ${CYAN}http://$DOMAIN${NC}"
        echo -e "  HTTPS: ${CYAN}https://$DOMAIN${NC}"
        echo
    else
        echo -e "${BOLD}Access URL:${NC}"
        echo -e "  Local: ${CYAN}http://127.0.0.1:$PORT${NC}"
        echo
    fi

    echo -e "${BOLD}Useful Commands:${NC}"
    echo "  Start:   sudo systemctl start claudia-server"
    echo "  Stop:    sudo systemctl stop claudia-server"
    echo "  Status:  sudo systemctl status claudia-server"
    echo "  Logs:    sudo journalctl -u claudia-server -f"
    echo "  Restart: sudo systemctl restart claudia-server"
    echo

    echo -e "${BOLD}Firewall Status:${NC}"
    if command -v ufw &> /dev/null; then
        $SUDO_E ufw status numbered 2>&1 | head -10
    fi
    echo

    echo -e "${BOLD}Installation log:${NC} $LOG_FILE"
    echo
}

# ═══════════════════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════════════════

show_banner() {
    cat << 'BANNER'

    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║     ██████╗ ███████╗███╗   ██╗ ██████╗ ███████╗██████╗ ██████╗     ║
    ║     ██╔══██╗██╔════╝████╗  ██║██╔════╝ ██╔════╝██╔══██╗██╔══██╗    ║
    ║     ██████╔╝█████╗  ██╔██╗ ██║██║  ███╗█████╗  ██████╔╝██████╔╝    ║
    ║     ██╔══██╗██╔══╝  ██║╚██╗██║██║   ██║██╔══╝  ██╔══██╗██╔══██╗    ║
    ║     ██║  ██║███████╗██║ ╚████║╚██████╔╝███████╗██████╔╝██████╔╝    ║
    ║     ╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝ ╚═════╝ ╚══════╝╚═════╝ ╚═════╝     ║
    ║                                                               ║
    ║              Server Installation Script                       ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝

BANNER
}

main() {
    show_banner

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --port)
                PORT="$2"
                shift 2
                ;;
            --domain)
                DOMAIN="$2"
                shift 2
                ;;
            --dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --skip-deps)
                SKIP_DEPS=true
                shift
                ;;
            --skip-firewall)
                SKIP_FIREWALL=true
                shift
                ;;
            --skip-ssl)
                SKIP_SSL=true
                shift
                ;;
            --api-key)
                API_KEY="$2"
                shift 2
                ;;
            --api-url)
                CUSTOM_API_URL="$2"
                shift 2
                ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS]"
                echo
                echo "Options:"
                echo "  --port PORT         Server port (default: $DEFAULT_PORT)"
                echo "  --domain DOMAIN     Domain for nginx/SSL setup"
                echo "  --dir DIR           Installation directory"
                echo "  --api-key KEY       Anthropic API key"
                echo "  --api-url URL       Custom API URL"
                echo "  --skip-deps         Skip dependency installation"
                echo "  --skip-firewall     Skip firewall configuration"
                echo "  --skip-ssl          Skip SSL certificate request"
                echo "  --help, -h          Show this help"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                ;;
        esac
    done

    # Defaults
    PORT="${PORT:-$DEFAULT_PORT}"
    HOST="${HOST:-$DEFAULT_HOST}"
    INSTALL_DIR="${INSTALL_DIR:-$HOME/claudia-server}"
    CUSTOM_API_URL="${CUSTOM_API_URL:-}"

    # Pre-flight checks
    check_root
    detect_os

    echo
    log "Installation directory: $INSTALL_DIR"
    log "Server port: $PORT"
    log "Log file: $LOG_FILE"
    echo

    # Installation steps
    if [ "$SKIP_DEPS" != true ]; then
        install_system_deps
    fi

    install_rust
    install_nodejs
    install_claude
    clone_or_update_repo
    build_server
    build_frontend

    configure_claude_cli
    create_env_file

    if [ "$SKIP_FIREWALL" != true ]; then
        configure_firewall
    fi

    create_systemd_service

    if [ -n "$DOMAIN" ]; then
        configure_domain_ssl
    fi

    start_services
    print_summary
}

# Run
main "$@"
