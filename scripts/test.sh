#!/usr/bin/env bash
# Run the splitype test suite on Windows.
#
# rustc needs a larger main-thread stack to compile this crate's single
# giant test binary (a stack overflow otherwise aborts the build). This
# script copies rustc.exe, bumps its PE stack reserve with editbin, and
# points cargo at the patched copy — the original toolchain stays
# untouched.
#
# Usage: ./scripts/test.sh [cargo test args...]
set -euo pipefail

case "$(uname -s)" in
    MINGW64_NT-* | MSYS_NT-* | MINGW32_NT-*) ;;
    *)
        echo "This script targets Windows (git-bash / MSYS2)." >&2
        exit 1
        ;;
esac

RUSTUP_BIN="$(rustup which rustc)"
PATCHED="$(rustup which rustc).bigstack"
EDITBIN="/c/Program Files/Microsoft Visual Studio/18/Community/VC/Tools/MSVC/14.51.36231/bin/Hostx64/x64/editbin.exe"

# Re-patch whenever the toolchain's rustc changed (rustup update etc.).
if [ ! -f "$PATCHED" ] || [ "$RUSTUP_BIN" -nt "$PATCHED" ]; then
    cp "$RUSTUP_BIN" "$PATCHED"
    "$EDITBIN" /STACK:0x10000000,0x10000000 "$PATCHED" >/dev/null
    echo "Patched rustc stack: $PATCHED"
fi

export RUSTC="$PATCHED"
exec cargo test --bin splitype "$@"
