#!/bin/bash
set -e

# Define output directory
OUTPUT_DIR="rpm_dist"
mkdir -p $OUTPUT_DIR

echo "Starting RPM build using Podman (Fedora container)..."

# Run build in Podman
# We mount the current directory to /ws in the container
podman run --rm \
    -v $(pwd):/ws:z \
    -w /ws \
    fedora:39 \
    bash -c "
        echo 'Installing build dependencies...' && \
        dnf install -y rpm-build > /dev/null && \
        echo 'Building RPM...' && \
        # Define the build tree in a temporary directory
        mkdir -p /root/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS} && \
        # Run rpmbuild
        rpmbuild -bb rpm_package/whatsapp-desktop.spec \
            --define '_topdir /root/rpmbuild' && \
        # Copy the result back to the mounted volume
        cp /root/rpmbuild/RPMS/*/*.rpm /ws/$OUTPUT_DIR/
    "

echo "RPM build complete. Files are in $OUTPUT_DIR/"
ls -lh $OUTPUT_DIR/
