# Stage 1: Build Rust binaries (static musl)
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /src
# Copy standalone crate manifest and source
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
# Build lg-inspect and lg-grid binaries
RUN cargo build --release

# Stage 2: Ubuntu desktop with Xvfb + i3 + noVNC
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV DISPLAY=:1
ENV RESOLUTION=1280x1024x24

# Install desktop environment and tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Virtual display + window manager
    xvfb \
    i3 \
    # VNC + web access
    x11vnc \
    novnc \
    websockify \
    # X11 automation tools
    xdotool \
    wmctrl \
    x11-utils \
    scrot \
    # OCR
    tesseract-ocr \
    tesseract-ocr-jpn \
    tesseract-ocr-eng \
    # Java runtime (for GUI automation compatibility)
    default-jre-headless \
    # Utilities
    imagemagick \
    make \
    curl \
    jq \
    xz-utils \
    # Terminal
    xterm \
    # Fonts and theme
    fonts-dejavu-core \
    gnome-themes-extra \
    && rm -rf /var/lib/apt/lists/*

# GTK high contrast theme for OCR readability
RUN mkdir -p /root/.config/gtk-3.0 && \
    printf '[Settings]\ngtk-theme-name=HighContrast\ngtk-font-name=DejaVu Sans Mono 11\n' \
    > /root/.config/gtk-3.0/settings.ini

# Copy i3 config (OCR-optimized)
RUN mkdir -p /root/.config/i3
COPY docker/i3-config /root/.config/i3/config

# Copy Rust binaries from builder
COPY --from=builder /src/target/release/lg-inspect /usr/local/bin/
COPY --from=builder /src/target/release/lg-grid /usr/local/bin/

# Copy recipes
COPY docker/recipes/ /usr/local/lib/lg-recipes/

# Copy startup script
COPY docker/start-desktop.sh /usr/local/bin/start-desktop.sh

# Create shared directory for state
RUN mkdir -p /shared

EXPOSE 3000

CMD ["/usr/local/bin/start-desktop.sh"]
