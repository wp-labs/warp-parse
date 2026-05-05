// App-level registration of external connectors and features.
// This module provides unified registration functions that can be reused
// across multiple binary targets (wparse, wpgen, wproj, wprescue).
//
// By keeping these registrations out of the core library, we avoid
// feature-coupling the core with optional extension crates.
// All registration functions are safe to call multiple times as the
// registry ignores duplicate builders.

pub fn register_builtin() {
    wp_engine::sinks::register_builtin_sinks();
}

/// Register built-in sources and connectors for full functionality.
/// Used by wparse for complete runtime registration.
pub fn register_for_runtime() {
    // Register built-in sinks & source factories
    wp_engine::sinks::register_builtin_sinks();
    wp_engine::sources::file::register_factory_only();
    wp_engine::sources::syslog::register_syslog_factory();

    // Register optional external connectors
    register_optional_connectors();
}

/// Register external connector factories based on feature flags.
/// This function is community edition ready and includes Kafka, MySQL,
/// ClickHouse, Elasticsearch, Prometheus, VictoriaMetrics, VictoriaLogs,
/// Doris, Count, and HTTP connectors when enabled.
pub fn register_optional_connectors() {
    #[cfg(feature = "wp-connectors")]
    {
        use wp_engine::connectors::registry::{register_sink_factory, register_source_factory};

        // Kafka
        register_source_factory(wp_connectors::kafka::KafkaSourceFactory);
        register_sink_factory(wp_connectors::kafka::KafkaSinkFactory);
        // MySQL
        register_source_factory(wp_connectors::mysql::MySQLSourceFactory);
        register_sink_factory(wp_connectors::mysql::MySQLSinkFactory);
        // ClickHouse
        register_sink_factory(wp_connectors::clickhouse::ClickHouseSinkFactory);
        // Elasticsearch
        register_sink_factory(wp_connectors::elasticsearch::ElasticsearchSinkFactory);
        // Prometheus
        register_sink_factory(wp_connectors::prometheus::PrometheusFactory);
        // Doris
        register_sink_factory(wp_connectors::doris::DorisSinkFactory);
        // Count (debug/bench)
        register_source_factory(wp_connectors::count::CountSourceFactory);
        register_sink_factory(wp_connectors::count::CountSinkFactory);
        // VictoriaLogs
        register_sink_factory(wp_connectors::victorialogs::VictoriaLogSinkFactory);
        // VictoriaMetrics
        register_sink_factory(wp_connectors::victoriametrics::VictoriaMetricFactory);
        // HTTP
        register_source_factory(wp_connectors::http::HttpSourceFactory);
        register_sink_factory(wp_connectors::http::HttpSinkFactory);
    }

    #[cfg(feature = "wp-connectors-labs")]
    {
        use wp_engine::connectors::registry::{register_sink_factory, register_source_factory};

        // Dmdb (达梦数据库, experimental)
        register_source_factory(wp_connectors_labs::dmdb::DmdbSourceFactory);
        register_sink_factory(wp_connectors_labs::dmdb::DmdbSinkFactory);
    }
}
