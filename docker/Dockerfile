# ===========================================================================
# Warp Parse - Runtime Docker Image (Multi-arch)
# ===========================================================================

FROM ubuntu:24.04

ARG TARGETARCH

LABEL org.opencontainers.image.source="https://github.com/wp-labs/warp-parse"
LABEL org.opencontainers.image.description="High-performance flow data parsing and processing system"
LABEL org.opencontainers.image.licenses="ELv2"

# 安装运行时依赖。
# 其中 unixODBC 运行时用于 DMDB connector 在启动时通过 libloading 加载 libodbc.so。
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3t64 \
    libsqlite3-0 \
    zlib1g \
    libsasl2-2 \
    libzstd1 \
    libodbc2 \
    libodbcinst2 \
    unixodbc \
    odbcinst \
    libltdl7 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false wparse

# Copy pre-built binaries (provided via build context, arch-specific)
COPY ${TARGETARCH}/wparse /usr/local/bin/
COPY ${TARGETARCH}/wpgen /usr/local/bin/
COPY ${TARGETARCH}/wproj /usr/local/bin/
COPY ${TARGETARCH}/wprescue /usr/local/bin/

# Create data directory
RUN mkdir -p /data && chown wparse:wparse /data

# Copy default settings
COPY --chown=wparse:wparse default_setting/ /data/

USER wparse
WORKDIR /data

ENTRYPOINT ["wparse"]
CMD ["--help"]
