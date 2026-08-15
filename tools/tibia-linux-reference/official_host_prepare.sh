#!/usr/bin/env bash
set -euo pipefail

user="${1:-oteryn-tibia-ref}"

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 2
}

[[ "$(id -u)" -eq 0 ]] || fail "run as root on the dedicated Ubuntu host"
[[ "$user" =~ ^[a-z_][a-z0-9_-]*$ ]] || fail "dedicated username is invalid"
[[ -z "${CI:-}" && -z "${GITHUB_ACTIONS:-}" ]] || fail "CI runners are forbidden"
[[ ! -e /.dockerenv && ! -e /run/.containerenv ]] || fail "containers are forbidden"
! grep -qiE 'microsoft|wsl' /proc/version || fail "WSL is forbidden"
[[ -d /run/systemd/system ]] || fail "normal systemd Linux host or VM required"

# shellcheck disable=SC1091
source /etc/os-release
[[ "${ID:-}" == "ubuntu" ]] || fail "this preparation profile is intentionally Ubuntu-only"

apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  python3 \
  binutils \
  file \
  curl \
  ca-certificates \
  cryptsetup-bin \
  util-linux \
  libxcb-cursor0 \
  libx11-6 \
  libgl1 \
  mesa-utils \
  x11-utils

if ! id "$user" >/dev/null 2>&1; then
  useradd --create-home --user-group --shell /bin/bash "$user"
fi

# This account is deliberately not placed in sudo/admin/shared-runtime groups.
# No password or SSH credential is generated here: access material must be set
# locally on the host and must never enter Git, CI logs or chat.
for group in sudo adm docker lxd; do
  if getent group "$group" >/dev/null 2>&1 && id -nG "$user" | tr ' ' '\n' | grep -Fxq "$group"; then
    fail "dedicated user unexpectedly belongs to privileged/shared group: $group"
  fi
done

printf '{"result":"PASS","host_profile":"ubuntu-systemd","dedicated_user":"%s","packages_installed":true,"access_credential_generated":false,"evidence_volume_configured":false}\n' "$user"
