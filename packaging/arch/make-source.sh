#!/bin/sh
# Make the source tarball for the PKGBUILD from the committed tree (HEAD),
# put its checksum in the PKGBUILD, then build the package:
#   packaging/arch/make-source.sh && (cd packaging/arch && makepkg -f)
set -e
cd "$(dirname "$0")"
ver=$(sed -n 's/^pkgver=//p' PKGBUILD)
git -C ../.. archive --format=tar.gz --prefix="microkit-emu-$ver/" -o "$PWD/microkit-emu-$ver.tar.gz" HEAD
sum=$(sha256sum "microkit-emu-$ver.tar.gz" | cut -d' ' -f1)
sed -i "s/^sha256sums=.*/sha256sums=('$sum')/" PKGBUILD
echo "microkit-emu-$ver.tar.gz  $sum"
