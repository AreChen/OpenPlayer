#!/usr/bin/env bash
set -euo pipefail
sudo apt-get install -y build-essential clang cmake pkg-config libmpv-dev libssl-dev \
  libx11-xcb-dev libxcb-shape0-dev libxcb-xkb-dev libxkbcommon-x11-dev libwayland-dev \
  libasound2-dev libfontconfig1-dev libvulkan-dev libdbus-1-dev libudev-dev \
  patchelf desktop-file-utils file squashfs-tools mesa-vulkan-drivers
