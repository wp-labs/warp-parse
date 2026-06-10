# ===========================================================================
# Warp Parse - Runtime Docker Image (Multi-arch)
# ===========================================================================

FROM ubuntu:24.04

ARG TARGETARCH

LABEL org.opencontainers.image.source="https://github.com/wp-labs/warp-parse"
LABEL org.opencontainers.image.description="High-performance flow data parsing and processing system"
LABEL org.opencontainers.image.licenses="ELv2"

# 安装运行时依赖。
# 其中 unixODBC 运行时用于 DMDB connector 在启动时通过 libloading 加载 ODBC Driver Manager。
# 达梦运行时还会按库名查找 `libodbc.so` / `libodbcinst.so`，这里在镜像内补齐无版本软链。
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
    && ODBC_SO="$(find /usr/lib -path '*/libodbc.so.2' | head -n1)" \
    && test -n "$ODBC_SO" \
    && ln -sf "$(basename "$ODBC_SO")" "$(dirname "$ODBC_SO")/libodbc.so" \
    && ODBCINST_SO="$(find /usr/lib -path '*/libodbcinst.so.2' | head -n1)" \
    && test -n "$ODBCINST_SO" \
    && ln -sf "$(basename "$ODBCINST_SO")" "$(dirname "$ODBCINST_SO")/libodbcinst.so" \
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
