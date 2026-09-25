#!/usr/bin/env bash
set -euo pipefail

# Run from the repository root so VHS Source/Output paths are predictable.
cd "$(dirname "$0")/.."
if [[ "${1:-}" != "--local" ]]; then
    exec docker run --rm -v "$PWD:/data" -w /data \
        -v demand-vhs-target:/data/target \
        --entrypoint bash vhs assets/record.sh --local "$@"
fi
shift
if [[ $# -eq 0 ]]; then
    set -- assets/*.tape assets/themes/*.tape
fi

# Compile once, before VHS starts: compiler output and build times never enter
# the recordings. The Docker volume keeps Linux binaries out of the host target.
cargo build --examples
recording_dir=$(mktemp -d)
trap 'rm -rf "$recording_dir"' EXIT
for tape in "$@"; do
    # Render to a temporary file so an encoder failure cannot leave a stale GIF
    # looking like a successful recording (some VHS versions exit zero).
    vhs "$tape" --output "$recording_dir/demo.gif"
    if [[ ! -s "$recording_dir/demo.gif" ]]; then
        echo "VHS did not produce a GIF for $tape" >&2
        exit 1
    fi
    mv "$recording_dir/demo.gif" "${tape%.tape}.gif"
done
