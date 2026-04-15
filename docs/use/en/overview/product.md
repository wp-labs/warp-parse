# Warp Parse Product Overview

## Positioning

Warp Parse is a high-performance Rust engine for log, event, and security data ingestion. It focuses on:

- multi-source ingestion
- high-throughput parsing and transformation
- rule-driven routing
- operational simplicity through single-binary deployment

Typical users include security platforms, observability teams, data platforms, and real-time risk-control systems.

## Core Value

- High throughput and low latency for real-time or near-real-time ingestion
- Programmable rules through WPL and OML
- Unified connector model for sources and sinks
- File-based configuration and CLI-oriented operations
- Friendly to private deployments and controlled environments

## Good Fit

- security log ingestion and normalization
- structured processing for Nginx, application, and API gateway logs
- front-end cleansing before Kafka, Elasticsearch, or ClickHouse
- archive, disaster-recovery, and replay preparation

## Not A Standalone Fit

- complex stateful stream processing
- large-scale window aggregation or joins
- scheduled batch orchestration

Those scenarios are usually better combined with systems such as Flink, Spark, or Airflow.

## Main Components

- `wparse`: primary runtime for batch and daemon execution
- `wpgen`: helper tool for rule and configuration generation
- `wproj`: project management, validation, and runtime admin entry
- `wprescue`: rescue data handling tool

## Quick Start

Show CLI help:

```bash
wparse --help
wpgen --help
wproj --help
wprescue --help
```

## Related Docs

- Runtime admin usage: [../operations/admin.md](../operations/admin.md)
- Chinese counterpart: [../zh/overview/product.md](../zh/overview/product.md)
