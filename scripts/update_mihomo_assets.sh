#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZIP_DIR="${ROOT_DIR}/resources/zip"
CHECKSUMS_PATH="${ROOT_DIR}/checksums.txt"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

PROXY_URL="${MIHOMO_PROXY_URL:-http://127.0.0.1:7890}"
RELEASE_REF="${MIHOMO_RELEASE_REF:-latest}"
REPO_BASE_URL="https://github.com/MetaCubeX/mihomo/releases"

mkdir -p "${ZIP_DIR}"

log() {
  printf '%s\n' "$*"
}

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi

log "[1/4] Resolving release via ${PROXY_URL}"
if [[ "${RELEASE_REF}" == "latest" ]]; then
  curl --fail --silent --show-error --location --head \
    --proxy "${PROXY_URL}" \
    "${REPO_BASE_URL}/latest" \
    -o "${TMP_DIR}/latest_headers.txt"

  TAG_NAME="$(
    python3 - "${TMP_DIR}/latest_headers.txt" <<'PY'
import re
import sys
from pathlib import Path

headers = Path(sys.argv[1]).read_text(errors="ignore")
matches = re.findall(r"^location:\s*https://github\.com/MetaCubeX/mihomo/releases/tag/([^\r\n]+)$", headers, re.IGNORECASE | re.MULTILINE)
if not matches:
    raise SystemExit("Failed to resolve latest release tag")
print(matches[-1].strip())
PY
  )"
else
  TAG_NAME="${RELEASE_REF}"
fi

CHECKSUMS_URL="${REPO_BASE_URL}/download/${TAG_NAME}/checksums.txt"
log "      Release: ${TAG_NAME}"
ASSET_SOURCE="${TMP_DIR}/asset_source.txt"
CHECKSUMS_HTTP_CODE="$(
  curl --silent --show-error --location --write-out '%{http_code}' \
  --proxy "${PROXY_URL}" \
  "${CHECKSUMS_URL}" \
  --output "${CHECKSUMS_PATH}"
)"
if [[ "${CHECKSUMS_HTTP_CODE}" == "200" ]]; then
  cp "${CHECKSUMS_PATH}" "${ASSET_SOURCE}"
  log "[2/4] checksums.txt downloaded"
else
  rm -f "${CHECKSUMS_PATH}"
  log "[2/4] checksums.txt unavailable for ${TAG_NAME}, using release assets page"
  curl --fail --silent --show-error --location \
    --proxy "${PROXY_URL}" \
    "${REPO_BASE_URL}/expanded_assets/${TAG_NAME}" \
    -o "${ASSET_SOURCE}"
fi

python3 - "${ASSET_SOURCE}" "${TMP_DIR}/selection.json" "${TAG_NAME}" <<'PY'
import json
import re
import sys
from pathlib import Path

asset_source = Path(sys.argv[1]).read_text()
output = Path(sys.argv[2])
tag_name = sys.argv[3]

targets = [
    {
        "key": "linux-amd64-compatible",
        "pattern": r"^mihomo-linux-amd64-compatible-.*\.gz$",
        "preferred": r"^mihomo-linux-amd64-compatible-v[0-9].*\.gz$",
        "target": "mihomo-linux-amd64-compatible.gz",
    },
    {
        "key": "linux-arm64",
        "pattern": r"^mihomo-linux-arm64-.*\.gz$",
        "preferred": r"^mihomo-linux-arm64-v[0-9].*\.gz$",
        "target": "mihomo-linux-arm64.gz",
    },
    {
        "key": "darwin-arm64",
        "pattern": r"^mihomo-darwin-arm64-.*\.gz$",
        "preferred": r"^mihomo-darwin-arm64-v[0-9].*\.gz$",
        "target": "mihomo-darwin-arm64.gz",
    },
    {
        "key": "windows-386",
        "pattern": r"^mihomo-windows-386-.*\.zip$",
        "preferred": r"^mihomo-windows-386-v[0-9].*\.zip$",
        "target": "mihomo-windows-386.zip",
    },
]

result = {
    "tag_name": tag_name,
    "downloads": [],
}

asset_names = []
for line in asset_source.splitlines():
    parts = line.split()
    if len(parts) >= 2 and parts[-1].startswith("./mihomo-"):
        asset_names.append(parts[-1].removeprefix("./"))

if not asset_names:
    asset_names = re.findall(
        rf"/MetaCubeX/mihomo/releases/download/{re.escape(tag_name)}/([^\"/?]+)",
        asset_source,
    )

for name in dict.fromkeys(asset_names):
    for target in targets:
        if re.match(target["pattern"], name):
            result["downloads"].append(
                {
                    "key": target["key"],
                    "asset_name": name,
                    "target_name": target["target"],
                    "url": f"https://github.com/MetaCubeX/mihomo/releases/download/{tag_name}/{name}",
                }
            )
            break

selected_downloads = []
for target in targets:
    matches = [item for item in result["downloads"] if item["key"] == target["key"]]
    if not matches:
        continue
    preferred = [item for item in matches if re.match(target["preferred"], item["asset_name"])]
    pool = preferred or matches
    pool.sort(key=lambda item: (item["asset_name"].count("-"), len(item["asset_name"])))
    selected_downloads.append(pool[0])

result["downloads"] = selected_downloads

missing = [target["key"] for target in targets if target["key"] not in {item["key"] for item in result["downloads"]}]
if missing:
    raise SystemExit("Missing assets in selected release: " + ", ".join(missing))

output.write_text(json.dumps(result, indent=2))
PY

log "[3/4] Downloading target assets"
python3 - "${TMP_DIR}/selection.json" "${ZIP_DIR}" "${PROXY_URL}" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

selection = json.loads(Path(sys.argv[1]).read_text())
zip_dir = Path(sys.argv[2])
proxy = sys.argv[3]

for item in selection["downloads"]:
    target_path = zip_dir / item["target_name"]
    print(f"      {item['target_name']} <= {item['asset_name']}")
    subprocess.run(
        [
            "curl",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proxy",
            proxy,
            item["url"],
            "--output",
            str(target_path),
        ],
        check=True,
    )
PY

while IFS= read -r old_file; do
  base_name="$(basename "${old_file}")"
  case "${base_name}" in
    mihomo-linux-amd64-compatible.gz|mihomo-linux-arm64.gz|mihomo-darwin-arm64.gz|mihomo-windows-386.zip)
      ;;
    mihomo-*)
      rm -f "${old_file}"
      ;;
  esac
done < <(find "${ZIP_DIR}" -maxdepth 1 -type f \( -name 'mihomo-*.gz' -o -name 'mihomo-*.zip' \))

log "[4/4] Done"
log "      Output: ${ZIP_DIR}"
log "      Release: ${TAG_NAME}"
