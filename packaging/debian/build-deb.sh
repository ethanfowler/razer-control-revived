#!/bin/bash
# Build the .deb from release binaries: run from the repository root after
# `cargo build --release` in razer_control_gui. Writes
# razercontrol-revived_<version>_amd64.deb to the current directory.
#
# Usage: packaging/debian/build-deb.sh <version>

set -euo pipefail

VERSION="${1:?usage: $0 <version>}"
SRC=razer_control_gui
PKG="razercontrol-revived_${VERSION}_amd64"

rm -rf "$PKG"
install -Dm755 "$SRC/target/release/razer-settings" "$PKG/usr/bin/razer-settings"
install -Dm755 "$SRC/target/release/razer-cli" "$PKG/usr/bin/razer-cli"
install -Dm755 "$SRC/target/release/daemon" "$PKG/usr/bin/razer-daemon"
install -Dm755 "$SRC/target/release/razer-helper" "$PKG/usr/libexec/razercontrol/razer-helper"
install -Dm755 "$SRC/data/setup-nvidia-powerd.sh" "$PKG/usr/libexec/razercontrol/setup-nvidia-powerd"
install -Dm644 "$SRC/data/devices/laptops.json" "$PKG/usr/share/razercontrol/laptops.json"
install -Dm644 "$SRC/data/gui/com.encomjp.razer-settings.desktop" "$PKG/usr/share/applications/com.encomjp.razer-settings.desktop"
install -Dm644 "$SRC/data/gui/icon.png" "$PKG/usr/share/icons/hicolor/256x256/apps/com.github.encomjp.razercontrol.png"
install -Dm644 "$SRC/data/udev/70-razercontrol-hidraw.rules" "$PKG/usr/lib/udev/rules.d/70-razercontrol-hidraw.rules"
install -Dm644 "$SRC/data/services/systemd/razercontrol.service" "$PKG/usr/lib/systemd/user/razercontrol.service"
install -Dm644 "$SRC/data/polkit/com.github.encomjp.razercontrol.helper.policy" "$PKG/usr/share/polkit-1/actions/com.github.encomjp.razercontrol.helper.policy"
install -Dm644 "$SRC/data/polkit/70-razercontrol-nvidia-powerd.rules" "$PKG/usr/share/polkit-1/rules.d/70-razercontrol-nvidia-powerd.rules"

mkdir -p "$PKG/DEBIAN"
cat > "$PKG/DEBIAN/control" <<EOF
Package: razercontrol-revived
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Depends: libgtk-4-1, libadwaita-1-0, libdbus-1-3, libhidapi-hidraw0, pkexec
Maintainer: EncomJP <encomjp@users.noreply.github.com>
Homepage: https://github.com/encomjp/razer-control-revived
Description: Razer Laptop Control - Revived
 A Linux userspace application to control Razer Blade laptops.
 No kernel modules (DKMS) required! Features a modern GTK4/libadwaita
 interface for fan control, power modes, CPU power limits, GPU clock caps,
 keyboard lighting, and battery health optimization.
EOF

# The user service is started for whoever ran `sudo apt install`, and enabled
# for everyone at their next login.
cat > "$PKG/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
udevadm control --reload-rules || true
udevadm trigger --subsystem-match=hidraw --action=change || true
/usr/libexec/razercontrol/setup-nvidia-powerd || true
if [ -d /run/systemd/system ]; then
  systemctl --global enable razercontrol.service || true
  if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != root ]; then
    systemctl --user -M "$SUDO_USER@" daemon-reload || true
    systemctl --user -M "$SUDO_USER@" restart razercontrol.service || true
  fi
fi
EOF

cat > "$PKG/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e
if [ -d /run/systemd/system ] && [ "$1" = remove ]; then
  systemctl --global disable razercontrol.service || true
  if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != root ]; then
    systemctl --user -M "$SUDO_USER@" stop razercontrol.service || true
  fi
fi
EOF
chmod 755 "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/prerm"

dpkg-deb --build --root-owner-group "$PKG"
rm -rf "$PKG"
