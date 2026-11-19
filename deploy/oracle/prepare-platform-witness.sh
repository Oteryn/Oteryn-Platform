#!/usr/bin/env bash
set -euo pipefail
umask 077
[[ $EUID == 0 ]] || { echo 'Run as root.' >&2; exit 1; }

directory=/opt/oteryn-oracle/native-evidence-witness
if [[ ! -e $directory && ! -L $directory ]]; then
    install -d -o 65532 -g 65532 -m 0700 -- "$directory"
fi
[[ -d $directory && ! -L $directory && $(stat -c '%u %g %a' -- "$directory") == '65532 65532 700' ]] \
    || { echo 'Unsafe native evidence witness directory.' >&2; exit 1; }
