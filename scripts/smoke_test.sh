#!/usr/bin/env bash
set -euo pipefail

binary="${1:?binary path is required}"

"${binary}" --version
