#!/bin/sh
# Razer Laptop Control - set up nvidia-powerd (NVIDIA Dynamic Boost) if needed.
#
# Ubuntu and others ship nvidia-powerd but leave its service and D-Bus policy
# as examples under /usr/share/doc. Install NVIDIA's own copies so the Dynamic
# Boost toggle can work. An existing unit is left alone, the service isn't
# enabled (the toggle decides), and nothing here is fatal. Run as root.

[ -x /usr/bin/nvidia-powerd ] || exit 0
[ "$(systemctl show -p LoadState --value nvidia-powerd.service 2>/dev/null)" = loaded ] && exit 0

unit="$(ls -1 /usr/share/doc/nvidia-*/nvidia-powerd.service 2>/dev/null | sort -V | tail -n1)"
dbus_conf="$(ls -1 /usr/share/doc/nvidia-*/nvidia-dbus.conf 2>/dev/null | sort -V | tail -n1)"
if [ -z "$unit" ] || [ -z "$dbus_conf" ]; then
    echo "nvidia-powerd has no service files to install; Dynamic Boost will be unavailable"
    exit 0
fi

echo "Setting up nvidia-powerd (Dynamic Boost, left disabled)..."
install -Dm644 "$dbus_conf" /etc/dbus-1/system.d/nvidia-dbus.conf &&
    install -Dm644 "$unit" /etc/systemd/system/nvidia-powerd.service &&
    systemctl daemon-reload &&
    systemctl reload dbus ||
    echo "WARNING: couldn't set up nvidia-powerd; Dynamic Boost will be unavailable"
exit 0
