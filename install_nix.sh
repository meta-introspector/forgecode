#!/bin/sh

# This script installs the Nix package manager on your system by
# downloading a binary distribution and running its installer script
# (which in turn creates and populates /nix).

{ # Prevent execution if this script was only partially downloaded
oops() {
    echo "$0:" "$@" >&2
    exit 1
}

umask 0022

tmpDir="$(mktemp -d -t nix-binary-tarball-unpack.XXXXXXXXXX || \
          oops "Can't create temporary directory for downloading the Nix binary tarball")"
cleanup() {
    rm -rf "$tmpDir"
}
trap cleanup EXIT INT QUIT TERM

require_util() {
    command -v "$1" > /dev/null 2>&1 ||
        oops "you do not have '$1' installed, which I need to $2"
}

case "$(uname -s).$(uname -m)" in
    Linux.x86_64)
        hash=eafe5042404e818505e28c5ca3d0885f3ec45c31f955489a25bb38258f87560e
        path=b6wklxj1vmj03czm5j87bxw8djk31ssk/nix-2.34.7-x86_64-linux.tar.xz
        system=x86_64-linux
        ;;
    Linux.i?86)
        hash=0695f4b7b289c3f66c1a76447db164a2f96492a0d1cf8f2a7b2caaa8bfab08fb
        path=6qx5lygpv3p051a2ahiixcn81h6yvqwv/nix-2.34.7-i686-linux.tar.xz
        system=i686-linux
        ;;
    Linux.aarch64)
        hash=f1cee64ae7a02330c6421924c28f597c41813f2214ff108622087d8056378b08
        path=xlh3qck3ghjmgjpmr5ncxdf848ipb1b7/nix-2.34.7-aarch64-linux.tar.xz
        system=aarch64-linux
        ;;
    Linux.armv6l)
        hash=a7152bcc4489eae2e688895a6d070362d03c46720649a9418ca423e200ef0f5f
        path=qf5zgz70f33p0d2c6rlsgsx06wlv899j/nix-2.34.7-armv6l-linux.tar.xz
        system=armv6l-linux
        ;;
    Linux.armv7l)
        hash=758e8fb802fb83b571c1cbac906d77dff67e6724cf09110f87d1f84dde432685
        path=w20pbp9a8m6ic4ay3c0i69sk5pis8vq2/nix-2.34.7-armv7l-linux.tar.xz
        system=armv7l-linux
        ;;
    Linux.riscv64)
        hash=562290f2753ecb9e25e53a5b814473e80dd8f1ab504427f422cb455287084c18
        path=6jb2mw8nnpa2117q77p988q8ranpq7j3/nix-2.34.7-riscv64-linux.tar.xz
        system=riscv64-linux
        ;;
    Darwin.x86_64)
        hash=8bf3dadfd65be182ad3141b1224bbc82e0f2a61d4f36781938b5e6ede029c2a3
        path=a2c8n0k97vsh05kmbvi97b31f9myjx1z/nix-2.34.7-x86_64-darwin.tar.xz
        system=x86_64-darwin
        ;;
    Darwin.arm64|Darwin.aarch64)
        hash=71e18301c4ea78c667f2753159156b5bdb899993720e8aa7bcca97e8312d3d6b
        path=k02965k69hhik2d1f5424nsvm4cnwdsy/nix-2.34.7-aarch64-darwin.tar.xz
        system=aarch64-darwin
        ;;
    *) oops "sorry, there is no binary distribution of Nix for your platform";;
esac

# Use this command-line option to fetch the tarballs using nar-serve or Cachix
if [ "${1:-}" = "--tarball-url-prefix" ]; then
    if [ -z "${2:-}" ]; then
        oops "missing argument for --tarball-url-prefix"
    fi
    url=${2}/${path}
    shift 2
else
    url=https://releases.nixos.org/nix/nix-2.34.7/nix-2.34.7-$system.tar.xz
fi

tarball=$tmpDir/nix-2.34.7-$system.tar.xz

require_util tar "unpack the binary tarball"
if [ "$(uname -s)" != "Darwin" ]; then
    require_util xz "unpack the binary tarball"
fi

if command -v curl > /dev/null 2>&1; then
    fetch() { curl --fail -L "$1" -o "$2"; }
elif command -v wget > /dev/null 2>&1; then
    fetch() { wget "$1" -O "$2"; }
else
    oops "you don't have wget or curl installed, which I need to download the binary tarball"
fi

echo "downloading Nix 2.34.7 binary tarball for $system from '$url' to '$tmpDir'..."
fetch "$url" "$tarball" || oops "failed to download '$url'"

if command -v sha256sum > /dev/null 2>&1; then
    hash2="$(sha256sum -b "$tarball" | cut -c1-64)"
elif command -v shasum > /dev/null 2>&1; then
    hash2="$(shasum -a 256 -b "$tarball" | cut -c1-64)"
elif command -v openssl > /dev/null 2>&1; then
    hash2="$(openssl dgst -r -sha256 "$tarball" | cut -c1-64)"
else
    oops "cannot verify the SHA-256 hash of '$url'; you need one of 'shasum', 'sha256sum', or 'openssl'"
fi

if [ "$hash" != "$hash2" ]; then
    oops "SHA-256 hash mismatch in '$url'; expected $hash, got $hash2"
fi

unpack=$tmpDir/unpack
mkdir -p "$unpack"
tar -xJf "$tarball" -C "$unpack" || oops "failed to unpack '$url'"

script=$(echo "$unpack"/*/install)

[ -e "$script" ] || oops "installation script is missing from the binary tarball!"
export INVOKED_FROM_INSTALL_IN=1
"$script" "$@"

} # End of wrapping
