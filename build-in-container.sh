#!/bin/bash
set -e

CONTAINER_NAME="whatsapp-builder"
IMAGE_NAME="whatsapp-desktop-builder"
OUTPUT_DIR="./target/container-release"

echo "=== Building WhatsApp Desktop in Podman container ==="

# Build the container image
echo "[1/3] Building container image..."
podman build -t "$IMAGE_NAME" -f Containerfile .

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Extract the binary from the container
echo "[2/3] Extracting binary..."
podman create --name "$CONTAINER_NAME" "$IMAGE_NAME" true 2>/dev/null || true
podman cp "$CONTAINER_NAME:/app/target/release/whatsapp-desktop" "$OUTPUT_DIR/"
podman rm "$CONTAINER_NAME" 2>/dev/null || true

# Make it executable
chmod +x "$OUTPUT_DIR/whatsapp-desktop"

echo "[3/3] Done!"
echo ""
echo "Binary location: $OUTPUT_DIR/whatsapp-desktop"
echo "Binary size: $(du -h "$OUTPUT_DIR/whatsapp-desktop" | cut -f1)"
echo ""
echo "To run: $OUTPUT_DIR/whatsapp-desktop"
