FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Tauri v2 system dependencies + FFmpeg dev libraries in one layer
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Tauri requirements
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
    # FFmpeg dev libraries (reeln-media via ffmpeg-next)
    libavcodec-dev libavformat-dev libavfilter-dev libavdevice-dev \
    libavutil-dev libswscale-dev libswresample-dev \
    # Build essentials
    pkg-config build-essential curl ca-certificates git \
    && rm -rf /var/lib/apt/lists/*
