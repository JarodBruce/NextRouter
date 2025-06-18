#!/bin/bash

# Prometheus Setup Script
# This script downloads, installs, and configures Prometheus

set -e  # Exit on any error

# Configuration
PROMETHEUS_VERSION="2.45.0"
PROMETHEUS_USER="prometheus"
PROMETHEUS_HOME="/opt/prometheus"
PROMETHEUS_DATA="/var/lib/prometheus"
PROMETHEUS_CONFIG="/etc/prometheus"
WORKING_DIR=$(pwd)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING] $1${NC}"
}

# Check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        error "This script should not be run as root for security reasons"
        error "Please run as a regular user with sudo privileges"
        exit 1
    fi
}

# Check if sudo is available
check_sudo() {
    if ! command -v sudo &> /dev/null; then
        error "sudo is required but not installed"
        exit 1
    fi
    
    if ! sudo -v; then
        error "sudo privileges required"
        exit 1
    fi
}

# Check system requirements
check_requirements() {
    log "Checking system requirements..."
    
    # Check OS
    if [[ "$OSTYPE" != "linux-gnu"* ]]; then
        error "This script is designed for Linux systems"
        exit 1
    fi
    
    # Check architecture
    ARCH=$(uname -m)
    case $ARCH in
        x86_64)
            PROMETHEUS_ARCH="amd64"
            ;;
        aarch64)
            PROMETHEUS_ARCH="arm64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    # Check required commands
    for cmd in wget tar; do
        if ! command -v $cmd &> /dev/null; then
            error "$cmd is required but not installed"
            exit 1
        fi
    done
    
    log "System requirements check passed"
}

# Create prometheus user
create_user() {
    log "Creating prometheus user..."
    
    if id "$PROMETHEUS_USER" &>/dev/null; then
        warning "User $PROMETHEUS_USER already exists, skipping creation"
    else
        sudo useradd --no-create-home --shell /bin/false $PROMETHEUS_USER
        log "User $PROMETHEUS_USER created"
    fi
}

# Create directories
create_directories() {
    log "Creating directories..."
    
    sudo mkdir -p $PROMETHEUS_HOME
    sudo mkdir -p $PROMETHEUS_DATA
    sudo mkdir -p $PROMETHEUS_CONFIG
    sudo mkdir -p $PROMETHEUS_CONFIG/rules
    
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_HOME
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_DATA
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/rules
    
    log "Directories created and permissions set"
}

# Download and install Prometheus
download_prometheus() {
    log "Downloading Prometheus v$PROMETHEUS_VERSION..."
    
    PROMETHEUS_TARBALL="prometheus-$PROMETHEUS_VERSION.linux-$PROMETHEUS_ARCH.tar.gz"
    DOWNLOAD_URL="https://github.com/prometheus/prometheus/releases/download/v$PROMETHEUS_VERSION/$PROMETHEUS_TARBALL"
    
    cd /tmp
    wget -q "$DOWNLOAD_URL"
    
    if [[ ! -f "$PROMETHEUS_TARBALL" ]]; then
        error "Failed to download Prometheus"
        exit 1
    fi
    
    log "Extracting Prometheus..."
    tar xf "$PROMETHEUS_TARBALL"
    
    PROMETHEUS_DIR="prometheus-$PROMETHEUS_VERSION.linux-$PROMETHEUS_ARCH"
    
    # Copy binaries
    sudo cp "$PROMETHEUS_DIR/prometheus" $PROMETHEUS_HOME/
    sudo cp "$PROMETHEUS_DIR/promtool" $PROMETHEUS_HOME/
    
    # Copy console files
    sudo cp -r "$PROMETHEUS_DIR/consoles" $PROMETHEUS_CONFIG/
    sudo cp -r "$PROMETHEUS_DIR/console_libraries" $PROMETHEUS_CONFIG/
    
    # Set permissions
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_HOME/prometheus
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_HOME/promtool
    sudo chown -R $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/consoles
    sudo chown -R $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/console_libraries
    
    # Create symlinks for easy access
    sudo ln -sf $PROMETHEUS_HOME/prometheus /usr/local/bin/prometheus
    sudo ln -sf $PROMETHEUS_HOME/promtool /usr/local/bin/promtool
    
    # Cleanup
    rm -rf /tmp/"$PROMETHEUS_DIR" /tmp/"$PROMETHEUS_TARBALL"
    
    log "Prometheus installed successfully"
}

# Setup configuration
setup_config() {
    log "Setting up Prometheus configuration..."
    
    # Copy prometheus.yml from current directory
    if [[ -f "$WORKING_DIR/prometheus.yml" ]]; then
        sudo cp "$WORKING_DIR/prometheus.yml" $PROMETHEUS_CONFIG/
        sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/prometheus.yml
        log "prometheus.yml copied to $PROMETHEUS_CONFIG/"
    else
        warning "prometheus.yml not found in current directory, creating default config"
        create_default_config
    fi
    
    # Copy alert rules if they exist
    if [[ -f "$WORKING_DIR/alert_rules.yml" ]]; then
        sudo cp "$WORKING_DIR/alert_rules.yml" $PROMETHEUS_CONFIG/rules/
        sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/rules/alert_rules.yml
        log "alert_rules.yml copied to $PROMETHEUS_CONFIG/rules/"
    fi
    
    # Validate configuration
    if sudo -u $PROMETHEUS_USER $PROMETHEUS_HOME/promtool check config $PROMETHEUS_CONFIG/prometheus.yml; then
        log "Prometheus configuration is valid"
    else
        error "Prometheus configuration validation failed"
        exit 1
    fi
}

# Create default configuration if prometheus.yml doesn't exist
create_default_config() {
    log "Creating default Prometheus configuration..."
    
    sudo tee $PROMETHEUS_CONFIG/prometheus.yml > /dev/null <<EOF
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yml"

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'rust-app'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s
EOF
    
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/prometheus.yml
}

# Create systemd service
create_service() {
    log "Creating systemd service..."
    
    sudo tee /etc/systemd/system/prometheus.service > /dev/null <<EOF
[Unit]
Description=Prometheus
Wants=network-online.target
After=network-online.target

[Service]
User=$PROMETHEUS_USER
Group=$PROMETHEUS_USER
Type=simple
ExecStart=$PROMETHEUS_HOME/prometheus \\
    --config.file=$PROMETHEUS_CONFIG/prometheus.yml \\
    --storage.tsdb.path=$PROMETHEUS_DATA \\
    --web.console.templates=$PROMETHEUS_CONFIG/consoles \\
    --web.console.libraries=$PROMETHEUS_CONFIG/console_libraries \\
    --web.listen-address=0.0.0.0:9090 \\
    --web.enable-lifecycle

Restart=always
RestartSec=10
StartLimitInterval=0

[Install]
WantedBy=multi-user.target
EOF
    
    sudo systemctl daemon-reload
    sudo systemctl enable prometheus
    
    log "Systemd service created and enabled"
}

# Start Prometheus
start_prometheus() {
    log "Starting Prometheus..."
    
    sudo systemctl start prometheus
    
    # Wait a moment for startup
    sleep 3
    
    if sudo systemctl is-active --quiet prometheus; then
        log "Prometheus started successfully"
        log "Prometheus is available at: http://localhost:9090"
    else
        error "Failed to start Prometheus"
        log "Check logs with: sudo journalctl -u prometheus -f"
        exit 1
    fi
}

# Show status
show_status() {
    log "Prometheus Setup Complete!"
    echo
    echo "=== Status ==="
    sudo systemctl status prometheus --no-pager -l
    echo
    echo "=== Service Management ==="
    echo "Start:   sudo systemctl start prometheus"
    echo "Stop:    sudo systemctl stop prometheus"
    echo "Restart: sudo systemctl restart prometheus"
    echo "Status:  sudo systemctl status prometheus"
    echo "Logs:    sudo journalctl -u prometheus -f"
    echo
    echo "=== Configuration ==="
    echo "Config file: $PROMETHEUS_CONFIG/prometheus.yml"
    echo "Data dir:    $PROMETHEUS_DATA"
    echo "Rules dir:   $PROMETHEUS_CONFIG/rules"
    echo
    echo "=== Access ==="
    echo "Web UI: http://localhost:9090"
    echo "API:    http://localhost:9090/api/v1/"
    echo
    echo "=== Validate Config ==="
    echo "promtool check config $PROMETHEUS_CONFIG/prometheus.yml"
}

# Main function
main() {
    log "Starting Prometheus setup..."
    
    check_root
    check_sudo
    check_requirements
    create_user
    create_directories
    download_prometheus
    setup_config
    create_service
    start_prometheus
    show_status
    
    log "Prometheus setup completed successfully!"
}

# Run main function
main "$@"