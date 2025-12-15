#!/usr/bin/env bash
# 简单的直通 wrapper，直接调用真正的 rustc，可用来覆盖外部的 sccache 设置
exec "$@"
