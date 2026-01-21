#!/bin/bash
set -e

# Installation script for Monero Web Coordinator

INSTALL_DIR="/opt/coordinator"
SERVICE_USER="coordinator"

echo "Installing Monero Web Coordinator..."

# Create user if not exists
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd --system --no-create-home --shell /bin/false "$SERVICE_USER"
    echo "Created user: $SERVICE_USER"
fi

# Create directories
mkdir -p "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR/logs"

# Copy binary (assumes built binary exists)
if [ -f "target/release/monero-web-coordinator" ]; then
    cp target/release/monero-web-coordinator "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/monero-web-coordinator"
    echo "Binary installed"
else
    echo "Error: Build the project first with 'cargo build --release'"
    exit 1
fi

# Copy config if not exists
if [ ! -f "$INSTALL_DIR/config.toml" ]; then
    cp config.example.toml "$INSTALL_DIR/config.toml"
    echo "Config copied - please edit $INSTALL_DIR/config.toml"
fi

# Set ownership
chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"

# Install systemd service
cp deploy/coordinator.service /etc/systemd/system/
systemctl daemon-reload

echo ""
echo "Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Edit $INSTALL_DIR/config.toml"
echo "  2. Start service: systemctl start coordinator"
echo "  3. Enable on boot: systemctl enable coordinator"
echo "  4. Check status: systemctl status coordinator"
