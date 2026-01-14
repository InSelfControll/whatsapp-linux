#!/bin/bash
# Run WhatsApp Desktop in a container with display access

podman run --rm -it \
    --security-opt label=disable \
    -e DISPLAY=$DISPLAY \
    -v /tmp/.X11-unix:/tmp/.X11-unix:ro \
    -v $HOME/.config/whatsapp-desktop:/root/.config/whatsapp-desktop \
    --device /dev/dri \
    --name whatsapp-running \
    whatsapp-desktop-builder \
    /app/target/release/whatsapp-desktop
