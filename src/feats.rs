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
/// ClickHouse, Elasticsearch, and Prometheus connectors when enabled.
pub fn register_optional_connectors() {
    // Initialize runtime registries if needed
    wp_engine::connectors::startup::init_runtime_registries();

    // NOTE:
    // 目前大部分社区连接器工厂已可与 wp-engine 注册接口兼容，恢复注册。
    // 仍未恢复的 connector 会按具体原因单独告警。

    #[cfg(any(feature = "community", feature = "kafka"))]
    {
        wp_engine::connectors::registry::register_source_factory(
            wp_connectors::kafka::KafkaSourceFactory,
        );
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::kafka::KafkaSinkFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "doris"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::doris::DorisSinkFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "mysql"))]
    {
        wp_engine::connectors::registry::register_source_factory(
            wp_connectors::mysql::MySQLSourceFactory,
        );
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::mysql::MySQLSinkFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "postgres"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::postgres::PostgresSinkFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "victoriametrics"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::victoriametrics::VictoriaMetricFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "victorialogs"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::victorialogs::VictoriaLogSinkFactory,
        );
    }

    // ClickHouse connector is not exported from wp-connectors crate root currently.
    #[cfg(any(feature = "community", feature = "clickhouse"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::clickhouse::ClickHouseSinkFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "http"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::http::HttpSinkFactory,
        );
    }

    #[cfg(any(feature = "community", feature = "elasticsearch"))]
    {
        wp_engine::connectors::registry::register_sink_factory(
            wp_connectors::elasticsearch::ElasticsearchSinkFactory,
        );
    }

    // Prometheus module currently does not export a public sink factory type.
    #[cfg(any(feature = "community", feature = "prometheus"))]
    {
        wp_log::warn_ctrl!(
            "skip prometheus connector registration: public factory type is not exported"
        );
    }

    // Enterprise features placeholder
    #[cfg(feature = "wp-enterprise")]
    {
        // Enterprise-specific connectors would be registered here
    }
}
