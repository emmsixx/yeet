#!/bin/sh
# Install a published GitHub Release. No sudo, shell-profile edits, or telemetry.
set -eu

fail() { printf 'yeet installer: %s\n' "$*" >&2; exit 1; }
usage() {
    cat <<'EOF'
Usage: sh install.sh [--version VERSION] [--bin-dir DIRECTORY]

Install the latest stable release, or select an exact version (with or without v).
Defaults: YEET_VERSION=latest, YEET_INSTALL_DIR=$HOME/.local/bin
Linux and macOS, x86_64 and ARM64. Windows: download the release ZIP manually.
Rerun to upgrade a script-managed install. Use your package manager for its installs.
EOF
}

version=${YEET_VERSION:-latest}
bin_dir=${YEET_INSTALL_DIR:-${HOME:?HOME must be set}/.local/bin}
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) [ "$#" -ge 2 ] || fail '--version requires a value'; version=$2; shift 2 ;;
        --bin-dir) [ "$#" -ge 2 ] || fail '--bin-dir requires a value'; bin_dir=$2; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) fail "unknown argument: $1" ;;
    esac
done
case "$bin_dir" in /*) ;; *) fail '--bin-dir must be an absolute path' ;; esac

for command in curl tar mktemp uname awk; do
    command -v "$command" >/dev/null 2>&1 || fail "required command missing: $command"
done
if command -v sha256sum >/dev/null 2>&1; then
    hash_program=sha256sum
elif command -v shasum >/dev/null 2>&1; then
    hash_program=shasum
else
    fail 'sha256sum or shasum is required'
fi

case "$(uname -s)" in
    Linux) platform=unknown-linux-gnu ;;
    Darwin) platform=apple-darwin ;;
    *) fail 'unsupported operating system; see the release downloads' ;;
esac
case "$(uname -m)" in
    x86_64|amd64) architecture=x86_64 ;;
    arm64|aarch64) architecture=aarch64 ;;
    *) fail 'unsupported CPU architecture' ;;
esac

repository=https://github.com/emmsixx/yeet
fetch() {
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 --fail --silent \
        --show-error --location --retry 3 --connect-timeout 15 --max-time 180 "$@"
}
if [ "$version" = latest ]; then
    resolved=$(fetch --output /dev/null --write-out '%{url_effective}' "$repository/releases/latest") ||
        fail 'cannot resolve latest release; use --version for a prerelease'
    case "$resolved" in
        "$repository"/releases/tag/*) version=${resolved##*/} ;;
        *) fail 'no published stable release found; use --version for a prerelease' ;;
    esac
fi
case "$version" in v*) ;; *) version="v$version" ;; esac
printf '%s\n' "$version" | LC_ALL=C awk '
    /^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$/ { valid=1 }
    END { exit !(valid && NR == 1) }
' || fail 'invalid version; expected a release such as v0.1.0 or v0.1.0-alpha.1'

target="$architecture-$platform"
name="yeet-$version-$target"
archive="$name.tar.gz"
temporary=$(mktemp -d "${TMPDIR:-/tmp}/yeet-install.XXXXXX")
replacement=
cleanup() {
    rm -rf "$temporary"
    if [ -n "$replacement" ]; then rm -f "$replacement"; fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

base="$repository/releases/download/$version"
fetch --output "$temporary/$archive" "$base/$archive" || fail 'archive download failed'
fetch --output "$temporary/checksum" "$base/$archive.sha256" || fail 'checksum download failed'
expected=$(awk -v archive="$archive" '
    NF == 2 && $2 == archive && length($1) == 64 && $1 !~ /[^0-9a-f]/ { count++; hash=$1 }
    END { if (NR != 1 || count != 1) exit 1; print hash }
' "$temporary/checksum") || fail 'invalid checksum file'
if [ "$hash_program" = sha256sum ]; then
    actual=$(sha256sum "$temporary/$archive")
else
    actual=$(shasum -a 256 "$temporary/$archive")
fi
actual=${actual%% *}
[ "$actual" = "$expected" ] || fail 'checksum mismatch; nothing was installed'

# Extract only the binary to stdout: archive paths/symlinks cannot write outside temp.
tar -xOzf "$temporary/$archive" "$name/yeet" > "$temporary/yeet" || fail 'cannot extract binary'
[ -s "$temporary/yeet" ] || fail 'archive contains an empty binary'
chmod 755 "$temporary/yeet"
installed_version=$("$temporary/yeet" --version) || fail 'binary cannot run on this system'
[ "$installed_version" = "yeet ${version#v}" ] || fail 'binary version does not match release'

mkdir -p "$bin_dir"
destination="$bin_dir/yeet"
[ ! -L "$destination" ] || fail 'destination is a symlink; update it with its original installer'
[ ! -d "$destination" ] || fail 'destination is a directory'
# Install via a sibling temporary file so replacement is atomic on this filesystem.
replacement=$(mktemp "$bin_dir/.yeet.XXXXXX")
cat "$temporary/yeet" > "$replacement"
chmod 755 "$replacement"
mv -f "$replacement" "$destination"
replacement=
printf 'Installed %s to %s\n' "$installed_version" "$destination"
printf 'Ensure %s is on your PATH.\n' "$bin_dir"
