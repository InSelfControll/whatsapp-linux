{
  description = "WhatsApp Web Desktop Wrapper";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            autoPatchelfHook
            copyDesktopItems
            wrapGAppsHook
          ];

          buildInputs = with pkgs; [
            openssl
            glib
            gtk3
            webkitgtk_4_1
            libsoup_3
            gdk-pixbuf
            libappindicator-gtk3
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
            gst_all_1.gst-plugins-ugly
          ];

          # Copy icon and desktop file
          postInstall = ''
            install -Dm644 assets/icon.png $out/share/icons/hicolor/256x256/apps/whatsapp-desktop.png
            
            mkdir -p $out/share/applications
            cat > $out/share/applications/whatsapp-desktop.desktop <<EOF
            [Desktop Entry]
            Name=WhatsApp Desktop
            Comment=WhatsApp Web Wrapper
            Exec=$out/bin/whatsapp-desktop
            Icon=whatsapp-desktop
            Terminal=false
            Type=Application
            Categories=Network;InstantMessaging;Chat;
            StartupWMClass=WhatsApp
            EOF
          '';

          meta = with pkgs.lib; {
            description = manifest.description;
            homepage = "https://github.com/yourusername/whatsapp-desktop";
            license = licenses.mit;
            maintainers = [ ];
            mainProgram = "whatsapp-desktop";
            platforms = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ];

          buildInputs = with pkgs; [
            openssl
            glib
            gtk3
            webkitgtk_4_1
            libsoup_3
            gdk-pixbuf
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
          ];
        };
      }
    );
}
