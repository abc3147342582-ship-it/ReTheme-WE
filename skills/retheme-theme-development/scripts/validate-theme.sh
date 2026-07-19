#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "Usage: validate-theme.sh <theme-directory-or-source.zip>" >&2
    exit 2
fi

INPUT=$1
if [ -d "$INPUT" ]; then
    MODE=--directory
elif [ -f "$INPUT" ]; then
    MODE=--source
else
    echo "Theme path does not exist: $INPUT" >&2
    exit 2
fi

if command -v retheme-theme-validator >/dev/null 2>&1; then
    exec retheme-theme-validator "$MODE" "$INPUT"
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SEARCH=$SCRIPT_DIR
while [ "$SEARCH" != "/" ]; do
    if [ -f "$SEARCH/crates/theme-validator/Cargo.toml" ]; then
        exec cargo run --quiet --manifest-path "$SEARCH/crates/theme-validator/Cargo.toml" -- "$MODE" "$INPUT"
    fi
    SEARCH=$(dirname "$SEARCH")
done

echo "retheme-theme-validator is not installed and no ReTheme source checkout was found." >&2
exit 127
