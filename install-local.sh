#!/bin/bash
# Local installation script for razercontrol-revived

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/razer_control_gui"

if [ "$EUID" -eq 0 ]; then
    echo "Please do not run as root (sudo will be used where needed)"
    exit 1
fi

echo "Installing razercontrol-revived..."

RAZER_SETTINGS_BIN="$BUILD_DIR/target/release/razer-settings"
RAZER_DAEMON_BIN="$BUILD_DIR/target/release/daemon"
RAZER_CLI_BIN="$BUILD_DIR/target/release/razer-cli"
RAZER_HELPER_BIN="$BUILD_DIR/target/release/razer-helper"

# Always build: cargo only rebuilds what changed, and skipping the build when
# binaries exist would install stale ones.
CARGO="$(command -v cargo || echo "$HOME/.cargo/bin/cargo")"
echo "Building release binaries..."
"$CARGO" build --release --manifest-path "$BUILD_DIR/Cargo.toml"

# Install binaries (consistent with deb/rpm package names)
echo "Installing binaries to /usr/bin..."
sudo install -Dm755 "$RAZER_SETTINGS_BIN" /usr/bin/razer-settings
sudo install -Dm755 "$RAZER_DAEMON_BIN" /usr/bin/razer-daemon
sudo install -Dm755 "$RAZER_CLI_BIN" /usr/bin/razer-cli
# Root helper for CPU power limits, turbo and GPU clock caps, run by the daemon
# through pkexec; its polkit policy lets the user at the machine run it
sudo install -Dm755 "$RAZER_HELPER_BIN" /usr/libexec/razercontrol/razer-helper
sudo install -Dm644 "$BUILD_DIR/data/polkit/com.github.encomjp.razercontrol.helper.policy" /usr/share/polkit-1/actions/com.github.encomjp.razercontrol.helper.policy

# Install desktop file
echo "Installing desktop entry..."
sudo install -Dm644 "$BUILD_DIR/data/gui/com.encomjp.razer-settings.desktop" /usr/share/applications/com.encomjp.razer-settings.desktop

echo "Installing SVG icon..."
sudo mkdir -p /usr/share/icons/hicolor/scalable/apps/
sudo install -Dm644 "$BUILD_DIR/data/gui/com.github.encomjp.razercontrol.svg" /usr/share/icons/hicolor/scalable/apps/com.github.encomjp.razercontrol.svg

# Install udev rules
echo "Installing udev rules..."
sudo rm -f /etc/udev/rules.d/99-hidraw-permissions.rules
sudo install -Dm644 "$BUILD_DIR/data/udev/70-razercontrol-hidraw.rules" /etc/udev/rules.d/70-razercontrol-hidraw.rules
# Files from earlier builds that gave a razercontrol group sysfs access
sudo rm -f /etc/udev/rules.d/70-razercontrol-powercap.rules \
    /etc/tmpfiles.d/razercontrol.conf \
    /etc/polkit-1/rules.d/70-razercontrol-gpu-clocks.rules \
    /usr/share/polkit-1/actions/com.github.encomjp.razercontrol.gpu-clocks.policy \
    /usr/libexec/razercontrol/razer-gpu-clocks
# nvidia-powerd (Dynamic Boost): the user at the machine may start and stop it
sudo install -Dm644 "$BUILD_DIR/data/polkit/70-razercontrol-nvidia-powerd.rules" /etc/polkit-1/rules.d/70-razercontrol-nvidia-powerd.rules
sudo install -Dm755 "$BUILD_DIR/data/setup-nvidia-powerd.sh" /usr/libexec/razercontrol/setup-nvidia-powerd
sudo /usr/libexec/razercontrol/setup-nvidia-powerd

# Install systemd user service
echo "Installing systemd user service..."
sudo install -Dm644 "$BUILD_DIR/data/services/systemd/razercontrol.service" /usr/lib/systemd/user/razercontrol.service

# Install device configuration
echo "Installing device configuration..."
sudo mkdir -p /usr/share/razercontrol
sudo install -Dm644 "$BUILD_DIR/data/devices/laptops.json" /usr/share/razercontrol/laptops.json

# Create config directory
mkdir -p ~/.local/share/razercontrol

# Reload udev and systemd
echo "Reloading udev rules..."
sudo udevadm control --reload-rules
sudo udevadm trigger

echo "Reloading systemd user daemon..."
systemctl --user daemon-reload

# Enable and start the user service
echo "Enabling and starting razercontrol daemon..."
systemctl --user enable razercontrol.service
systemctl --user restart razercontrol.service


# Validating icon cache
if command -v gtk-update-icon-cache &> /dev/null; then
    echo "Updating GTK icon cache..."
    sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi

if command -v kbuildsycoca5 &> /dev/null; then
    echo "Updating KDE configuration cache..."
    kbuildsycoca5 --noincremental &> /dev/null || true
elif command -v kbuildsycoca6 &> /dev/null; then
    echo "Updating KDE configuration cache..."
    kbuildsycoca6 --noincremental &> /dev/null || true
fi

# Update Plasmoid if detected
PLASMOID_DIR="$HOME/.local/share/plasma/plasmoids/com.github.encomjp.razercontrol"
if [ -d "$PLASMOID_DIR" ]; then
    echo "Updating KDE Plasmoid..."
    cp -r "$BUILD_DIR/kde-widget/package/"* "$PLASMOID_DIR/" 2>/dev/null || true
fi

echo ""
echo "Installation complete!"

