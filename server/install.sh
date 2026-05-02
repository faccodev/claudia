#!/bin/bash
# Claudia Server Installation Script
# ==================================
# This script installs Claudia (server version) on a VPS
# It will:
# 1. Install Node.js (if not present)
# 2. Install Claude CLI
# 3. Build and install Claudia Server
# 4. Configure environment and API settings

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CLAUDIA_DIR="${CLAUDIA_DIR:-$HOME/.claudia}"
CLAUDIA_DATA_DIR="${CLAUDIA_DATA_DIR:-$HOME/.local/share/claudia}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/claudia-server}"
LOG_FILE="/tmp/claudia-install.log"

# Default values
DEFAULT_PORT=3000
DEFAULT_HOST="0.0.0.0"
DEFAULT_DOMAIN=""

# Functions
log() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
    exit 1
}

# Check if running as root for system installations
check_permissions() {
    if [ "$EUID" -eq 0 ]; then
        SUDO=""
        log "Running as root - will use system-wide installations"
    else
        SUDO="sudo"
        log "Running as user - will use user-wide installations"
    fi
}

# Detect OS
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        VER=$VERSION_ID
    else
        OS=$(uname -s)
        VER=$(uname -r)
    fi

    case $OS in
        ubuntu|debian|raspbian|linuxmint)
            PKG_MANAGER="apt-get"
            ;;
        fedora|rhel|centos|almalinux|rocky)
            PKG_MANAGER="dnf"
            ;;
        arch|manjaro)
            PKG_MANAGER="pacman"
            ;;
        alpine)
            PKG_MANAGER="apk"
            ;;
        darwin)
            PKG_MANAGER="brew"
            ;;
        *)
            PKG_MANAGER="unknown"
            ;;
    esac

    log "Detected OS: $OS $VER (package manager: $PKG_MANAGER)"
}

# Request password with sudo
request_password() {
    if [ "$EUID" -ne 0 ]; then
        echo -n "Enter your password for sudo: "
        read -rs PASSWORD
        echo
        export SUDO_PASSWORD="$PASSWORD"
    fi
}

# Execute command with sudo
run_as_sudo() {
    if [ -n "$SUDO_PASSWORD" ]; then
        echo "$SUDO_PASSWORD" | $SUDO -S "$@" >> "$LOG_FILE" 2>&1
    else
        $SUDO "$@"
    fi
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."

    local missing_deps=()

    # Check for basic tools
    for cmd in curl wget git; do
        if ! command -v $cmd &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done

    # Check for Rust (required for building)
    if ! command -v rustc &> /dev/null; then
        log "Rust not found - will install"
        missing_deps+=("rust")
    fi

    # Check for Node.js (required for some dependencies)
    if ! command -v node &> /dev/null; then
        log "Node.js not found - will install"
        missing_deps+=("node")
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        log "Missing dependencies: ${missing_deps[*]}"
    fi
}

# Install system dependencies
install_system_deps() {
    log "Installing system dependencies..."

    case $PKG_MANAGER in
        apt-get)
            run_as_sudo apt-get update
            run_as_sudo apt-get install -y \
                build-essential \
                pkg-config \
                libssl-dev \
                curl \
                wget \
                git \
                file \
                || error "Failed to install system dependencies"
            ;;
        dnf)
            run_as_sudo dnf install -y \
                gcc \
                gcc-c++ \
                make \
                pkg-config \
                openssl-devel \
                curl \
                wget \
                git \
                || error "Failed to install system dependencies"
            ;;
        pacman)
            run_as_sudo pacman -Sy --noconfirm \
                base-devel \
                pkg-config \
                openssl \
                curl \
                wget \
                git \
                || error "Failed to install system dependencies"
            ;;
        apk)
            run_as_sudo apk add --no-cache \
                build-base \
                pkgconfig \
                openssl-dev \
                curl \
                wget \
                git \
                || error "Failed to install system dependencies"
            ;;
        *)
            warn "Unknown package manager - skipping system dependencies"
            ;;
    esac

    success "System dependencies installed"
}

# Install Rust
install_rust() {
    if command -v rustc &> /dev/null; then
        log "Rust already installed: $(rustc --version)"
        return
    fi

    log "Installing Rust..."

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        || error "Failed to install Rust"

    # Source cargo environment
    source "$HOME/.cargo/env"

    success "Rust installed: $(rustc --version)"
}

# Install Node.js
install_nodejs() {
    if command -v node &> /dev/null; then
        log "Node.js already installed: $(node --version)"
        return
    fi

    log "Installing Node.js..."

    # Try to install via package manager first
    case $PKG_MANAGER in
        apt-get)
            # Install NodeSource repository
            curl -fsSL https://deb.nodesource.com/setup_lts.x | run_as_sudo bash -
            run_as_sudo apt-get install -y nodejs
            ;;
        dnf)
            curl -fsSL https://rpm.nodesource.com/setup_lts.x | bash -
            run_as_sudo dnf install -y nodejs
            ;;
        pacman)
            run_as_sudo pacman -Sy --noconfirm nodejs npm
            ;;
        apk)
            run_as_sudo apk add --no-cache nodejs npm
            ;;
    esac

    # Fallback to nvm if not installed
    if ! command -v node &> /dev/null; then
        log "Installing Node.js via NVM..."
        curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash -
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        nvm install --lts
    fi

    success "Node.js installed: $(node --version)"
}

# Install Claude CLI
install_claude() {
    log "Installing Claude CLI..."

    # Check if already installed
    if command -v claude &> /dev/null; then
        log "Claude CLI already installed: $(claude --version 2>/dev/null || echo 'version unknown')"
        read -p "Do you want to reinstall Claude CLI? [y/N]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    # Install via npm if available
    if command -v npm &> /dev/null; then
        log "Installing Claude CLI via npm..."
        npm install -g @anthropic-ai/claude-code \
            || warn "Failed to install Claude CLI via npm, trying alternative method"
    fi

    # Alternative: Install via official script
    if ! command -v claude &> /dev/null; then
        log "Trying alternative installation method..."

        # Try to install Claude CLI using npx
        npx -y @anthropic-ai/claude-code --version &> /dev/null || true

        # Or use npm to install globally
        npm install -g @anthropic-ai/claude-code --prefer-online \
            || warn "Could not install Claude CLI automatically. Please install manually: npm install -g @anthropic-ai/claude-code"
    fi

    if command -v claude &> /dev/null; then
        success "Claude CLI installed: $(claude --version 2>/dev/null || echo 'installed')"
    else
        warn "Claude CLI installation may have failed. Please verify manually."
    fi
}

# Configure Claude CLI
configure_claude() {
    log "Configuring Claude CLI..."

    # Request Anthropic API Key
    echo -e "${BLUE}[CONFIGURATION]${NC}"
    echo "To use Claude Code, you need an Anthropic API key."
    echo "You can get one at: https://console.anthropic.com/settings/keys"
    echo

    read -p "Enter your Anthropic API Key [leave empty to configure later]: " API_KEY
    if [ -n "$API_KEY" ]; then
        export ANTHROPIC_API_KEY="$API_KEY"
    fi

    # Request custom API URL (for alternative providers)
    read -p "Enter custom API URL (leave empty for default Anthropic) [optional]: " CUSTOM_API_URL

    # Request custom API Key for alternative provider
    read -p "Enter custom provider API Key [optional]: " CUSTOM_API_KEY

    # Configure Claude settings
    mkdir -p "$CLAUDIA_DIR"
    mkdir -p "$CLAUDIA_DATA_DIR"

    # Save configuration
    cat > "$CLAUDIA_DIR/settings.json" << EOF
{
    "api_key": "${API_KEY:-}",
    "api_url": "${CUSTOM_API_URL:-https://api.anthropic.com}",
    "custom_provider_key": "${CUSTOM_API_KEY:-}",
    "version": "1.0.0"
}
EOF

    # Configure Claude CLI config
    CLAUDE_CONFIG_DIR="${HOME}/.config/claude"
    mkdir -p "$CLAUDE_CONFIG_DIR"

    # Create or update Claude config
    if [ -f "$CLAUDE_CONFIG_DIR/config.json" ]; then
        log "Backing up existing Claude config..."
        cp "$CLAUDE_CONFIG_DIR/config.json" "$CLAUDE_CONFIG_DIR/config.json.bak"
    fi

    cat > "$CLAUDE_CONFIG_DIR/config.json" << EOF
{
    "api_key": "${API_KEY:-}",
    "api_url": "${CUSTOM_API_URL:-}",
    "max_tokens": 8192,
    "temperature": 1,
    "output_format": "stream-json"
}
EOF

    success "Claude CLI configured"
}

# Configure domain and SSL
configure_domain() {
    log "Configuring domain and SSL..."

    echo -e "${BLUE}[DOMAIN CONFIGURATION]${NC}"
    echo "Enter your domain name (e.g., claudia.example.com)"
    echo "Leave empty to skip domain setup."
    echo

    read -p "Domain name: " DOMAIN
    if [ -z "$DOMAIN" ]; then
        warn "Skipping domain configuration"
        return
    fi

    # Check if nginx is available
    if ! command -v nginx &> /dev/null && ! command -v nginx &> /dev/null; then
        log "Installing nginx..."
        case $PKG_MANAGER in
            apt-get)
                run_as_sudo apt-get install -y nginx certbot python3-certbot-nginx
                ;;
            dnf)
                run_as_sudo dnf install -y nginx certbot python3-certbot-nginx
                ;;
            pacman)
                run_as_sudo pacman -Sy --noconfirm nginx certbot
                ;;
            apk)
                run_as_sudo apk add --no-cache nginx certbot
                ;;
        esac
    fi

    # Create nginx configuration
    log "Creating nginx configuration..."
    NGINX_CONF="/etc/nginx/sites-available/claudia-server"
    NGINX_ENABLED="/etc/nginx/sites-enabled/claudia-server"

    cat > /tmp/claudia-nginx.conf << EOF
server {
    listen 80;
    server_name $DOMAIN;

    # Claudia API Server
    location / {
        proxy_pass http://127.0.0.1:$PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_cache_bypass \$http_upgrade;
        proxy_read_timeout 86400;
    }

    # WebSocket support
    location /ws {
        proxy_pass http://127.0.0.1:$PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 86400;
    }
}
EOF

    run_as_sudo cp /tmp/claudia-nginx.conf "$NGINX_CONF"
    run_as_sudo ln -sf "$NGINX_CONF" "$NGINX_ENABLED" 2>/dev/null || true

    # Remove default nginx site
    if [ -f /etc/nginx/sites-enabled/default ]; then
        run_as_sudo rm -f /etc/nginx/sites-enabled/default
    fi

    # Test nginx configuration
    if run_as_sudo nginx -t; then
        run_as_sudo systemctl reload nginx
        success "Nginx configured for domain $DOMAIN"
    else
        error "Nginx configuration test failed"
    fi

    # Request SSL certificate
    echo
    read -p "Do you want to configure SSL with Let's Encrypt? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if command -v certbot &> /dev/null; then
            log "Requesting SSL certificate..."
            run_as_sudo certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos --email "admin@$DOMAIN" 2>&1 | tee -a "$LOG_FILE" \
                || warn "SSL certificate request failed. You can run 'certbot --nginx -d $DOMAIN' manually."
        else
            warn "Certbot not installed. Run 'certbot --nginx -d $DOMAIN' manually after installation."
        fi
    fi

    success "Domain configuration complete"
    echo
    echo "Your Claudia Server is now accessible at:"
    echo -e "  ${BLUE}https://$DOMAIN${NC}"
    echo
}

# Build Claudia Server
build_claudia_server() {
    log "Building Claudia Server..."

    local BUILD_DIR="$INSTALL_DIR/build"

    mkdir -p "$BUILD_DIR"

    # Clone or update repository
    if [ -d "$INSTALL_DIR/.git" ]; then
        log "Updating existing installation..."
        cd "$INSTALL_DIR"
        git pull
    else
        log "Cloning Claudia repository..."
        git clone https://github.com/faccodev/claudia.git "$INSTALL_DIR" \
            || error "Failed to clone repository"
        cd "$INSTALL_DIR"
    fi

    # Build the server
    cd "$INSTALL_DIR/server"

    # Source cargo environment
    export PATH="$HOME/.cargo/bin:$PATH"

    log "Compiling server (this may take a few minutes)..."
    cargo build --release 2>&1 | tee -a "$LOG_FILE" \
        || error "Failed to build server"

    success "Claudia Server built successfully"
}

# Create systemd service (optional)
create_service() {
    log "Creating systemd service..."

    if command -v systemctl &> /dev/null && [ "$EUID" -eq 0 ]; then
        cat > /etc/systemd/system/claudia-server.service << EOF
[Unit]
Description=Claudia Server - Web UI for Claude Code
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR/server
Environment="RUST_LOG=info"
Environment="CLAUDIA_PORT=$PORT"
Environment="CLAUDIA_HOST=$HOST"
ExecStart=$INSTALL_DIR/server/target/release/claudia-server
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

        run_as_sudo systemctl daemon-reload
        run_as_sudo systemctl enable claudia-server
        success "Systemd service created"
    else
        warn "systemctl not available - skipping service creation"
    fi
}

# Build frontend
build_frontend() {
    log "Building frontend..."

    cd "$INSTALL_DIR"

    if command -v npm &> /dev/null; then
        npm install
        npm run build
        success "Frontend built"
    else
        warn "npm not available - frontend build skipped"
    fi
}

# Create environment file
create_env_file() {
    log "Creating environment configuration..."

    cat > "$INSTALL_DIR/.env" << EOF
# Claudia Server Configuration
# =============================

# Server settings
CLAUDIA_PORT=${PORT:-$DEFAULT_PORT}
CLAUDIA_HOST=${HOST:-$DEFAULT_HOST}
CLAUDIA_DATA_DIR=$CLAUDIA_DATA_DIR
CLAUDIA_DIR=$CLAUDIA_DIR

# Security
CLAUDIA_JWT_SECRET=${JWT_SECRET:-$(openssl rand -base64 32)}

# Claude CLI settings
ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY
ANTHROPIC_API_URL=${CUSTOM_API_URL:-https://api.anthropic.com}

# Logging
RUST_LOG=info
EOF

    success "Environment file created"
}

# Print summary
print_summary() {
    echo
    echo "=========================================="
    echo -e "${GREEN}Claudia Server Installation Complete!${NC}"
    echo "=========================================="
    echo
    echo "Configuration:"
    echo "  - Installation directory: $INSTALL_DIR"
    echo "  - Data directory: $CLAUDIA_DATA_DIR"
    echo "  - Config directory: $CLAUDIA_DIR"
    echo "  - Port: ${PORT:-$DEFAULT_PORT}"
    echo "  - Host: ${HOST:-$DEFAULT_HOST}"
    if [ -n "$DOMAIN" ]; then
    echo "  - Domain: $DOMAIN"
    fi
    echo
    echo "To start the server:"
    if command -v systemctl &> /dev/null && systemctl is-active --quiet claudia-server 2>/dev/null; then
        echo "  sudo systemctl start claudia-server"
    else
        echo "  cd $INSTALL_DIR/server"
        echo "  ./target/release/claudia-server"
    fi
    echo
    echo "To check logs:"
    echo "  journalctl -u claudia-server -f"
    echo
    echo "Access the web interface at:"
    echo -e "  ${BLUE}http://\$HOST:\$PORT${NC}"
    echo
    echo "Installation log saved to: $LOG_FILE"
    echo
}

# Main installation process
main() {
    echo
    echo "=========================================="
    echo -e "${GREEN}Claudia Server Installation${NC}"
    echo "=========================================="
    echo

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --port)
                PORT="$2"
                shift 2
                ;;
            --host)
                HOST="$2"
                shift 2
                ;;
            --dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --domain)
                DOMAIN="$2"
                shift 2
                ;;
            --skip-deps)
                SKIP_DEPS=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo
                echo "Options:"
                echo "  --port PORT     Set server port (default: $DEFAULT_PORT)"
                echo "  --host HOST     Set server host (default: $DEFAULT_HOST)"
                echo "  --domain DOMAIN Set domain name for nginx/SSL setup"
                echo "  --dir DIR       Set installation directory"
                echo "  --skip-deps     Skip dependency installation"
                echo "  --help          Show this help message"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                ;;
        esac
    done

    check_permissions
    detect_os

    # Interactive configuration
    echo -e "${BLUE}[CONFIGURATION]${NC}"
    echo "Please configure your installation:"
    echo

    read -p "Server port [$DEFAULT_PORT]: " INPUT_PORT
    PORT="${INPUT_PORT:-$DEFAULT_PORT}"

    read -p "Server host [$DEFAULT_HOST]: " INPUT_HOST
    HOST="${INPUT_HOST:-$DEFAULT_HOST}"

    read -p "Installation directory [$INSTALL_DIR]: " INPUT_DIR
    INSTALL_DIR="${INPUT_DIR:-$INSTALL_DIR}"

    read -p "Domain name (optional): " INPUT_DOMAIN
    DOMAIN="${INPUT_DOMAIN:-$DEFAULT_DOMAIN}"

    request_password

    echo
    log "Starting installation..."
    echo

    # Install dependencies
    if [ "$SKIP_DEPS" != true ]; then
        install_system_deps
        install_rust
        install_nodejs
    fi

    # Install and configure Claude
    install_claude
    configure_claude

    # Build server
    build_claudia_server

    # Build frontend (optional)
    if command -v npm &> /dev/null; then
        build_frontend
    fi

    # Create environment and service
    create_env_file
    create_service

    # Configure domain and SSL
    configure_domain

    print_summary
}

# Run main function
main "$@"
