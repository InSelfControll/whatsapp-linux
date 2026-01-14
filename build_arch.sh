#!/bin/bash
set -e

# Output directory for the final package
OUTPUT_DIR="arch_dist"
# Staging directory for the build context
STAGING_DIR="arch_staging"

mkdir -p $OUTPUT_DIR
mkdir -p $STAGING_DIR

echo "Preparing Arch Linux build context..."

# 1. Copy artifacts to staging
cp target/release/whatsapp-desktop $STAGING_DIR/
cp assets/icon.png $STAGING_DIR/

# 2. Generate the Desktop Entry file in staging
cat <<EOF > $STAGING_DIR/whatsapp-desktop.desktop
[Desktop Entry]
Name=WhatsApp Desktop
Comment=WhatsApp Web Wrapper
Exec=whatsapp-desktop
Icon=whatsapp-desktop
Terminal=false
Type=Application
Categories=Network;InstantMessaging;Chat;
StartupWMClass=WhatsApp
EOF

# 3. Create the PKGBUILD in staging
# We use 'SKIP' for sha256sums because we are copying local files we just built
cat <<EOF > $STAGING_DIR/PKGBUILD
# Maintainer: Ofir <ofir@example.com>
pkgname=whatsapp-desktop
pkgver=0.1.0
pkgrel=1
pkgdesc="WhatsApp Web Desktop Wrapper built with Rust and Wry"
arch=('x86_64')
url="https://github.com/yourusername/whatsapp-desktop"
license=('MIT')
depends=('gtk3' 'webkit2gtk-4.1' 'libsoup3' 'libappindicator-gtk3')
source=('whatsapp-desktop' 'icon.png' 'whatsapp-desktop.desktop')
sha256sums=('SKIP' 'SKIP' 'SKIP')

package() {
    # Install Binary
    install -Dm755 "\$srcdir/whatsapp-desktop" "\$pkgdir/usr/bin/whatsapp-desktop"
    
    # Install Icon
    install -Dm644 "\$srcdir/icon.png" "\$pkgdir/usr/share/icons/hicolor/256x256/apps/whatsapp-desktop.png"

    # Install Desktop Entry
    install -Dm644 "\$srcdir/whatsapp-desktop.desktop" "\$pkgdir/usr/share/applications/whatsapp-desktop.desktop"
}
EOF

echo "Starting Arch Linux build using Podman..."

# 4. Run the build in Podman
# We map the staging dir to /build inside the container
# We use 'base-devel' which has makepkg
# We must create a user 'builder' because makepkg refuses to run as root
podman run --rm \
    -v $(pwd)/$STAGING_DIR:/build:z \
    -w /build \
    archlinux:base-devel \
    bash -c "
        # Create a build user
        useradd -m builder && \
        # Give builder ownership of the build directory
        chown -R builder:builder /build && \
        # Switch to builder and run makepkg
        # -s: install missing deps (we use -d to skip this for speed as we have the binary)
        # -c: clean up
        # -f: force overwrite
        # -d: skip dependency checks (avoids downloading GBs of gtk libs just to zip a binary)
        su builder -c 'makepkg -cfd'
    "

# 5. Move artifacts to output directory
echo "Moving package to $OUTPUT_DIR..."
mv $STAGING_DIR/*.pkg.tar.zst $OUTPUT_DIR/

# Cleanup staging
rm -rf $STAGING_DIR

echo "Arch Linux package built successfully!"
ls -lh $OUTPUT_DIR/
