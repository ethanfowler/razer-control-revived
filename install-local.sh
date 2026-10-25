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
# CPU power limits: writable by the razercontrol group (takes effect after re-login)
sudo groupadd -f razercontrol
sudo usermod -aG razercontrol "$USER"
sudo install -Dm644 "$BUILD_DIR/data/udev/70-razercontrol-powercap.rules" /etc/udev/rules.d/70-razercontrol-powercap.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=powercap
# nvidia-powerd (Dynamic Boost): the razercontrol group may start and stop it
sudo install -Dm644 "$BUILD_DIR/data/polkit/70-razercontrol-nvidia-powerd.rules" /etc/polkit-1/rules.d/70-razercontrol-nvidia-powerd.rules
# CPU turbo: the razercontrol group may switch it
sudo install -Dm644 "$BUILD_DIR/data/tmpfiles/razercontrol.conf" /etc/tmpfiles.d/razercontrol.conf
sudo systemd-tmpfiles --create /etc/tmpfiles.d/razercontrol.conf

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

# The user service manager only picks up new groups when it restarts, which
# needs every session for this user to end (a reboot is the simplest way).
USER_MANAGER_PID="$(pgrep -u "$USER" -x systemd | head -n1)"
RAZERCONTROL_GID="$(getent group razercontrol | cut -d: -f3)"
if [ -n "$USER_MANAGER_PID" ] && ! grep -qw "$RAZERCONTROL_GID" <(grep '^Groups:' "/proc/$USER_MANAGER_PID/status"); then
    echo ""
    echo "NOTE: reboot (or log out of every session) once so the daemon gets the"
    echo "razercontrol group. Until then, CPU power limits and turbo can't be changed."
fi
