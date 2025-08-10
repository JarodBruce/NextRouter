#!/bin/bash

# Source utility functions
source "$(dirname "$0")/utils.sh"

# Prometheus settings
PROMETHEUS_VERSION="2.45.0"
PROMETHEUS_USER="prometheus"
PROMETHEUS_HOME="/opt/prometheus"
PROMETHEUS_DATA="/var/lib/prometheus"
PROMETHEUS_CONFIG="/etc/prometheus"
PROMETHEUS_YML_FILE="$(dirname "$0")/../prometheus.yml"

# Function to install Prometheus
install_prometheus() {
    log "Installing Prometheus..."

    # Stop Prometheus service if it is running
    if systemctl is-active --quiet prometheus; then
        log "Stopping Prometheus service before installation..."
        sudo systemctl stop prometheus
    fi
    
    if ! id "$PROMETHEUS_USER" &>/dev/null; then
        sudo useradd --no-create-home --shell /bin/false "$PROMETHEUS_USER"
        success "User $PROMETHEUS_USER created."
    else
        warning "User $PROMETHEUS_USER already exists."
    fi

    sudo mkdir -p "$PROMETHEUS_HOME" "$PROMETHEUS_DATA" "$PROMETHEUS_CONFIG/rules"
    sudo chown -R "$PROMETHEUS_USER:$PROMETHEUS_USER" "$PROMETHEUS_HOME" "$PROMETHEUS_DATA" "$PROMETHEUS_CONFIG"

    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64) PROMETHEUS_ARCH="amd64" ;;
        aarch64) PROMETHEUS_ARCH="arm64" ;;
        armv7l) PROMETHEUS_ARCH="armv7" ;;
        *) 
            warning "Unknown architecture: $arch. Defaulting to amd64."
            PROMETHEUS_ARCH="amd64"
            ;;
    esac
    
    local PROMETHEUS_TARBALL="prometheus-$PROMETHEUS_VERSION.linux-$PROMETHEUS_ARCH.tar.gz"
    local DOWNLOAD_URL="https://github.com/prometheus/prometheus/releases/download/v$PROMETHEUS_VERSION/$PROMETHEUS_TARBALL"
    
    cd /tmp || exit
    wget -q "$DOWNLOAD_URL"
    tar xf "$PROMETHEUS_TARBALL"
    
    local PROMETHEUS_DIR="prometheus-$PROMETHEUS_VERSION.linux-$PROMETHEUS_ARCH"
    sudo cp "$PROMETHEUS_DIR/prometheus" "$PROMETHEUS_DIR/promtool" "$PROMETHEUS_HOME/"
    sudo cp -r "$PROMETHEUS_DIR/consoles" "$PROMETHEUS_DIR/console_libraries" "$PROMETHEUS_CONFIG/"
    
    sudo chown -R "$PROMETHEUS_USER:$PROMETHEUS_USER" "$PROMETHEUS_HOME" "$PROMETHEUS_CONFIG"
    sudo ln -sf "$PROMETHEUS_HOME/prometheus" /usr/local/bin/prometheus
    sudo ln -sf "$PROMETHEUS_HOME/promtool" /usr/local/bin/promtool
    
    rm -rf "$PROMETHEUS_DIR" "$PROMETHEUS_TARBALL"
    success "Prometheus installation complete."
}

# Function to set up Prometheus configuration
setup_prometheus_config() {
    log "Setting up Prometheus configuration..."
    if [[ ! -f "$PROMETHEUS_YML_FILE" ]]; then
        error "prometheus.yml not found at $PROMETHEUS_YML_FILE"
        exit 1
    fi
    
    sudo cp "$PROMETHEUS_YML_FILE" "$PROMETHEUS_CONFIG/"
    sudo chown "$PROMETHEUS_USER:$PROMETHEUS_USER" "$PROMETHEUS_CONFIG/prometheus.yml"
    
    if ! sudo -u "$PROMETHEUS_USER" "$PROMETHEUS_HOME/promtool" check config "$PROMETHEUS_CONFIG/prometheus.yml"; then
        error "Prometheus configuration check failed."
        exit 1
    fi
    success "Prometheus configuration is valid."
}

# Function to create Prometheus systemd service
create_prometheus_service() {
    log "Creating Prometheus systemd service..."
    sudo tee /etc/systemd/system/prometheus.service > /dev/null <<EOF
[Unit]
Description=Prometheus Time Series Collection and Processing Server
Wants=network-online.target
After=network-online.target

[Service]
User=$PROMETHEUS_USER
Group=$PROMETHEUS_USER
Type=simple
Restart=always
ExecStart=$PROMETHEUS_HOME/prometheus 
    --config.file=$PROMETHEUS_CONFIG/prometheus.yml 
    --storage.tsdb.path=$PROMETHEUS_DATA 
    --web.console.templates=$PROMETHEUS_CONFIG/consoles 
    --web.console.libraries=$PROMETHEUS_CONFIG/console_libraries 
    --web.listen-address=0.0.0.0:9090 
    --web.enable-lifecycle

[Install]
WantedBy=multi-user.target
EOF
    sudo systemctl daemon-reload
    success "Prometheus service created."
}

# Function to install Rust
install_rust() {
    log "Installing Rust..."
    if command -v rustc &> /dev/null; then
        warning "Rust is already installed."
        return
    fi
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    success "Rust installed."
}

# Function to build and set up a Rust application as a service
setup_rust_service() {
    local service_name="$1"
    local service_desc="$2"
    local project_path="$3"
    local exec_args="$4"
    
    log "Setting up service: $service_name"
    cd "$project_path" || { error "Directory not found: $project_path"; return 1; }
    
    if ! cargo build --release; then
        error "Build failed for $service_name"
        return 1
    fi
    
    sudo tee "/etc/systemd/system/$service_name.service" > /dev/null <<EOF
[Unit]
Description=$service_desc
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$project_path
ExecStart=$project_path/target/release/$service_name $exec_args
Restart=always

[Install]
WantedBy=multi-user.target
EOF
    sudo systemctl daemon-reload
    success "$service_name service created."
}

# Function to start all monitoring services
start_monitoring_services() {
    log "Starting all monitoring services..."
    
    local services=("prometheus" "network-traffic-monitor" "packet-loss-monitor")
    for service in "${services[@]}"; do
        log "Starting $service..."
        sudo systemctl enable "$service"
        sudo systemctl restart "$service"
        if sudo systemctl is-active --quiet "$service"; then
            success "$service started successfully."
        else
            error "$service failed to start. Check logs with 'journalctl -u $service'."
        fi
    done
}
