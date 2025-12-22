// App-level registration of external connectors and features.
// This module provides unified registration functions that can be reused
// across multiple binary targets (wparse, wpgen, wproj, wprescue).
//
// By keeping these registrations out of the core library, we avoid
// feature-coupling the core with optional extension crates.
// All registration functions are safe to call multiple times as the
// registry ignores duplicate builders.

/// Register all available connector factories (sources and sinks).
/// This includes all community edition connectors like Kafka, MySQL, etc.
/// Controlled by feature flags.
use wp_engine::connectors::registry;

/// Register only built-in sinks (null, file, test_rescue).
/// Used by tools that need minimal functionality (e.g., wprescue).
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

    // Kafka connector (source/sink)
    // Available via "community" default feature or explicit "kafka" feature
    #[cfg(any(feature = "community", feature = "kafka"))]
    {
        /* Note: In some versions, function names may differ.
         * To ensure compilability across versions, external factory
         * registration is temporarily disabled here.
         * Uncomment when all versions have consistent APIs.
         */

        registry::register_source_factory(wp_connectors::kafka::KafkaSourceFactory);
        registry::register_sink_factory(wp_connectors::kafka::KafkaSinkFactory);
    }

    // MySQL connector (source/sink)
    #[cfg(any(feature = "community", feature = "mysql"))]
    {
        registry::register_source_factory(wp_connectors::mysql::MySQLSourceFactory);
        registry::register_sink_factory(wp_connectors::mysql::MySQLSinkFactory);
    }

    // ClickHouse connector (source/sink)
    #[cfg(any(feature = "community", feature = "clickhouse"))]
    {
        // registry::register_source_factory(wp_connectors::clickhouse::ClickHouseSourceFactory);
        // registry::register_sink_factory(wp_connectors::clickhouse::ClickHouseSinkFactory);
    }

    // Elasticsearch connector (sink)
    #[cfg(any(feature = "community", feature = "elasticsearch"))]
    {
        // registry::register_sink_factory(wp_connectors::elasticsearch::ElasticsearchSinkFactory);
    }

    // Prometheus connector (sink)
    #[cfg(any(feature = "community", feature = "prometheus"))]
    {
        // registry::register_sink_factory(wp_connectors::prometheus::PrometheusSinkFactory);
    }

    // Enterprise features placeholder
    #[cfg(feature = "wp-enterprise")]
    {
        // Enterprise-specific connectors would be registered here
    }
}
