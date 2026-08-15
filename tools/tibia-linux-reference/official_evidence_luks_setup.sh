#!/usr/bin/env bash
set -euo pipefail

user="${1:-}"
device="${2:-}"
mountpoint="${3:-/srv/oteryn-tibia-reference/evidence}"
confirm="${4:-}"
mapper="oteryn_tibia_evidence"
mapper_open=0
evidence_mounted=0

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 2
}

cleanup() {
  if [[ "$evidence_mounted" == 1 ]]; then
    umount "$mountpoint" 2>/dev/null || true
  fi
  if [[ "$mapper_open" == 1 ]]; then
    cryptsetup close "$mapper" 2>/dev/null || true
  fi
}

[[ "$(id -u)" -eq 0 ]] || fail "run as root on the dedicated Ubuntu host"
[[ -n "$user" && -n "$device" ]] || fail "usage: $0 USER /dev/BLANK-DISK [MOUNTPOINT] DESTROY:/dev/BLANK-DISK"
[[ "$user" =~ ^[a-z_][a-z0-9_-]*$ ]] || fail "dedicated username is invalid"
[[ "$confirm" == "DESTROY:$device" ]] || fail "exact destructive confirmation is required"
[[ -b "$device" ]] || fail "evidence device is not a block device"
id "$user" >/dev/null 2>&1 || fail "dedicated user does not exist"
[[ -z "${CI:-}" && -z "${GITHUB_ACTIONS:-}" ]] || fail "CI runners are forbidden"
[[ ! -e /.dockerenv && ! -e /run/.containerenv ]] || fail "containers are forbidden"
[[ -t 0 && -t 1 ]] || fail "interactive TTY is required so the LUKS passphrase cannot arrive through automation"

root_source="$(findmnt -n -o SOURCE /)"
if [[ "$root_source" == "$device"* ]]; then
  fail "refusing to touch the root-system block device"
fi

while read -r name mounted; do
  [[ -z "${mounted:-}" ]] || fail "device or child is mounted: $name -> $mounted"
done < <(lsblk -nrpo NAME,MOUNTPOINT "$device")

# Only a deliberately blank second virtual/physical disk is accepted. Existing
# signatures are never wiped automatically.
if wipefs -n "$device" | grep -q '[^[:space:]]'; then
  fail "device contains an existing signature; refusing to overwrite it"
fi

[[ ! -e "/dev/mapper/$mapper" ]] || fail "evidence mapper already exists"

trap cleanup EXIT
printf 'Creating LUKS2 on %s. cryptsetup will request the passphrase interactively.\n' "$device" >&2
cryptsetup luksFormat --type luks2 "$device"
cryptsetup open "$device" "$mapper"
mapper_open=1

mkfs.ext4 -m 0 -L OTERYN_TIBIA_EVIDENCE "/dev/mapper/$mapper"
install -d -m 700 "$mountpoint"
mount "/dev/mapper/$mapper" "$mountpoint"
evidence_mounted=1
primary_group="$(id -gn "$user")"
chown "$user:$primary_group" "$mountpoint"
chmod 700 "$mountpoint"

source_type="$(lsblk -ndo TYPE "/dev/mapper/$mapper" | head -n1)"
[[ "$source_type" == "crypt" ]] || fail "dm-crypt TYPE=crypt proof failed"
findmnt -T "$mountpoint" >/dev/null || fail "encrypted evidence mount is not active"

# The mapper intentionally remains open/mounted for the validation session.
mapper_open=0
evidence_mounted=0
trap - EXIT
printf '{"result":"PASS","encryption":"LUKS2/dm-crypt","block_device_type":"crypt","mountpoint":"%s","owner":"%s","persistent_automount_configured":false}\n' "$mountpoint" "$user"
