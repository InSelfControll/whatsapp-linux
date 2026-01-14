# Maintainer: Ofir <ofir@example.com>
pkgname=whatsapp-desktop
pkgver=0.1.0
pkgrel=1
pkgdesc="WhatsApp Web Desktop Wrapper built with Rust and Wry"
arch=('x86_64')
url="https://github.com/yourusername/whatsapp-desktop"
license=('MIT')
depends=('gtk3' 'webkit2gtk-4.1' 'libsoup3' 'libappindicator-gtk3')
makedepends=('cargo')
source=("path/to/source.tar.gz") # Update this with real release URL when available
sha256sums=('SKIP') # Use SKIP for local development, update for release

# For local development without a tarball, we just copy the current dir
# In a real AUR package, you'd pull from a git tag or tarball.
prepare() {
    # This is a placeholder. For local 'makepkg', ensure the source is correct.
    # If distributing via AUR, use source=("git+https://...")
    true
}

build() {
    # If building from local source (copying this PKGBUILD to project root)
    cd "$srcdir/.."
    cargo build --release --locked
}

package() {
    cd "$srcdir/.."
    
    install -Dm755 "target/release/whatsapp-desktop" "$pkgdir/usr/bin/whatsapp-desktop"
    install -Dm644 "assets/icon.png" "$pkgdir/usr/share/icons/hicolor/256x256/apps/whatsapp-desktop.png"

    mkdir -p "$pkgdir/usr/share/applications"
    cat > "$pkgdir/usr/share/applications/whatsapp-desktop.desktop" <<EOF
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
}
