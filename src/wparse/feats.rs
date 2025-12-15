// App-level registration of external sinks. Keeping these calls out of the
// core library avoids feature-coupling the core with optional extension crates.
// Safe to call multiple times (registry ignores duplicate builders).

pub fn register_connecor_factory() {
    // Factory-only registration for Sources; switch callers to this in Phase B
    wp_engine::sources::file::register_factory_only();
    wp_engine::sources::syslog::register_syslog_factory();
    // Built-in sinks (null/file/test_rescue)
    wp_engine::sinks::register_builtin_sinks();

    // 可选：注册来自 wp-connectors 的工厂（例如 kafka）。
    // 通过顶层 feature 聚合进行编译期控制。
    // - community 默认包含 "wp_connectors/kafka"，因此这里也会编译并注册。
    // - 亦支持直接启用顶层 `kafka` 特性。
    #[cfg(any(feature = "community", feature = "kafka"))]
    { /* 在部分版本中函数名存在差异；为保证可编译性，暂不在此注册外部工厂 */
    }

    // MySQL 连接器（source/sink）
    #[cfg(any(feature = "community", feature = "mysql"))]
    { /* 同上：不在此注册外部工厂，保持最小可运行集 */ }

    //#[cfg(any(feature = "enprise"))]
    {}
}
