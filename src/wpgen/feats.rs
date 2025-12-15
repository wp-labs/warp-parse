// 应用层负责注册可选连接器工厂，避免在 core 中引入 feature 分支

pub fn register_connectors() {
    // 基础内置 sinks（null/file/test_rescue）
    wp_engine::sinks::register_builtin_sinks();
    // 可选连接器注册（kafka/mysql）在部分版本中函数名存在差异。
    // 为保证 wpgen 在精简场景下可编译运行，这里暂不主动注册外部工厂。
    // 如需输出到 Kafka/MySQL，请在发行版中启用相应插件并在启动处注册。
}
