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
    // 依赖升级后，wp-engine/wp-motor 走 wp-connector-api 0.8，而 wp-connectors 仍是 0.7，
    // 这里的工厂类型不再兼容。先保留 feature 开关但不注册，确保主程序可编译运行；
    // 待 wp-connectors 升级到 0.8 线后恢复下面这些注册调用。

    #[cfg(any(feature = "community", feature = "kafka"))]
    {
        wp_log::warn_ctrl!("skip kafka connector registration: wp-connectors api version mismatch");
    }

    #[cfg(any(feature = "community", feature = "doris"))]
    {
        wp_log::warn_ctrl!("skip doris connector registration: wp-connectors api version mismatch");
    }

    #[cfg(any(feature = "community", feature = "mysql"))]
    {
        wp_log::warn_ctrl!("skip mysql connector registration: wp-connectors api version mismatch");
    }

    #[cfg(any(feature = "community", feature = "victoriametrics"))]
    {
        wp_log::warn_ctrl!(
            "skip victoriametrics connector registration: wp-connectors api version mismatch"
        );
    }

    #[cfg(any(feature = "community", feature = "victorialogs"))]
    {
        wp_log::warn_ctrl!(
            "skip victorialogs connector registration: wp-connectors api version mismatch"
        );
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
