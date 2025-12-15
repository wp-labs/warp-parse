#[allow(dead_code)]
pub fn register_sinks() {
    // 初始化引擎端连接器注册表（内置 sink/source 工厂 + 兼容导入）
    // 说明：wp-connector-api 不再提供全局注册表；统一通过 wp-engine 的 connectors::* 注册/获取
    wp_engine::connectors::startup::init_runtime_registries();
}

pub fn register_sources_factory_only() {
    // Factory-only registration for Sources (Phase B will switch callers to this)
    wp_engine::sources::file::register_factory_only();
}
