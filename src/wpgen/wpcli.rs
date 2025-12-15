use anyhow::Result;
use orion_conf::error::{ConfIOReason, OrionConfResult};
use orion_conf::TomlIO;
use orion_error::{ToStructError, UvsValidationFrom};
use std::path::{Path, PathBuf};
use wp_conf::engine::EngineConfig;
use wp_conf::loader::ConfDelegate;
use wp_conf::sinks::{load_connectors_for, ConnectorRec};
use wp_conf::structure::SinkInstanceConf;
use wp_data_model::model::fmt_def::TextFmt;
use wp_engine::facade::config::{WpGenConfig, WpGenResolved, WPGEN_TOML};
use wp_engine::facade::gen::{RuleGRA, SampleGRA};
use wp_engine::runtime::generator::{run_rule_direct, run_sample_direct};
use wp_error::run_error::RunResult;
use wp_log::info_launch;

fn conf_root(work_root: &str) -> PathBuf {
    Path::new(work_root).join("conf")
}

fn conf_path(work_root: &str, file_name: &str) -> PathBuf {
    conf_root(work_root).join(file_name)
}

fn work_path(work_root: &str, file_name: &str) -> PathBuf {
    Path::new(work_root).join(file_name)
}

pub fn load_wpgen_resolved(conf_name: &str, work_root: &str) -> OrionConfResult<WpGenResolved> {
    let conf = parse_wpgen_config(work_root, conf_name)?;
    let out_sink = resolve_out_sink(work_root, &conf)?;
    Ok(WpGenResolved { conf, out_sink })
}

fn parse_wpgen_config(work_root: &str, conf_name: &str) -> OrionConfResult<WpGenConfig> {
    let path = conf_path(work_root, conf_name);
    let conf = WpGenConfig::load_toml(&path)?;
    conf.validate()?;
    Ok(conf)
}

fn resolve_out_sink(work_root: &str, conf: &WpGenConfig) -> OrionConfResult<SinkInstanceConf> {
    let out_name = conf
        .output
        .name
        .clone()
        .unwrap_or_else(|| "gen_out".to_string());
    let conn_id = match conf.output.connect.clone() {
        Some(id) => id,
        None => {
            return ConfIOReason::from_validation(
                "wpgen.output.connect must be set (no default fallback)"
            )
            .err_result();
        }
    };
    let (_start_root, conn) = load_connector_by_id(work_root, &conn_id)?;
    let mut merged = merge_params_with_whitelist(&conn, &conf.output.params, &conn_id)?;
    if conn.kind == "tcp" {
        let unlimited = conf.generator.speed == 0;
        let has_explicit = merged.contains_key("max_backoff");
        if unlimited {
            if !has_explicit {
                merged.insert("max_backoff".into(), toml::Value::Boolean(true));
            }
        } else if has_explicit {
            merged.insert("max_backoff".into(), toml::Value::Boolean(false));
        }
    }
    let fmt = select_text_fmt(conn.kind.as_str(), &merged);
    let mut out = SinkInstanceConf::new_type(out_name, fmt, conn.kind.clone(), merged, None);
    out.connector_id = Some(conn_id);
    Ok(out)
}

fn load_connector_by_id(
    work_root: &str,
    conn_id: &str,
) -> OrionConfResult<(String, ConnectorRec)> {
    let wp_conf = EngineConfig::load_or_init(Path::new(work_root))?;
    let configured_path = Path::new(wp_conf.sinks_root());
    let resolved_root = if configured_path.is_absolute() {
        configured_path.to_path_buf()
    } else {
        work_path(work_root, wp_conf.sinks_root())
    };
    let start_root = resolved_root.to_string_lossy().to_string();
    let connectors = load_connectors_for(&start_root)?;
    let conn = connectors.get(conn_id).cloned().ok_or_else(|| {
        let mut known: Vec<String> = connectors.keys().cloned().collect();
        known.sort();
        if known.len() > 8 {
            known.truncate(8);
        }
        ConfIOReason::from_validation(format!(
            "wpgen.output.connect='{}' 不存在：自 start='{}' 向上最多 32 层搜索 'connectors/sink.d' 未找到该 id；已知 id: [{}]",
            conn_id,
            start_root,
            known.join(", ")
        ))
    })?;
    Ok((start_root, conn))
}

fn merge_params_with_whitelist(
    conn: &ConnectorRec,
    override_tbl: &toml::value::Table,
    conn_id: &str,
) -> OrionConfResult<toml::value::Table> {
    let mut merged = conn.params.clone();
    for (k, v) in override_tbl.iter() {
        if k == "params" || k == "params_override" {
            return ConfIOReason::from_validation(format!(
                "invalid nested table '{}' in output.params; set keys [{}] directly",
                k,
                conn.allow_override.join(", ")
            ))
            .err_result();
        }
        if !conn.allow_override.iter().any(|x| x == k) {
            return ConfIOReason::from_validation(format!(
                "override '{}' not allowed for connector '{}'; whitelist: [{}]",
                k,
                conn_id,
                conn.allow_override.join(", ")
            ))
            .err_result();
        }
        merged.insert(k.clone(), v.clone());
    }
    Ok(merged)
}

fn select_text_fmt(kind: &str, merged: &toml::value::Table) -> TextFmt {
    if kind == "file" {
        let s = merged.get("fmt").and_then(|v| v.as_str()).unwrap_or("json");
        TextFmt::from(s)
    } else {
        TextFmt::Json
    }
}

pub fn log_resolved_out_sink(resolved: &WpGenResolved) {
    let kind = resolved.out_sink.resolved_kind_str();
    let params = resolved.out_sink.resolved_params_table();
    info_launch!("wpgen out sink resolved: kind={}, params={:?}", kind, params);
}

pub fn conf_init(work_root: &str) -> OrionConfResult<()> {
    let path = conf_path(work_root, WPGEN_TOML);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let path_string = path.to_string_lossy().to_string();
    ConfDelegate::<WpGenConfig>::new(path_string.as_str())
        .init()
        .map(|_| ())
}

pub fn conf_clean(work_root: &str) -> OrionConfResult<()> {
    let path = conf_path(work_root, WPGEN_TOML);
    let path_string = path.to_string_lossy().to_string();
    ConfDelegate::<WpGenConfig>::new(path_string.as_str()).safe_clean()
}

pub fn conf_check(work_root: &str) -> OrionConfResult<()> {
    let _ = load_wpgen_resolved(WPGEN_TOML, work_root)?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct GenCleanReport {
    pub path: Option<String>,
    pub existed: bool,
    pub cleaned: bool,
    pub note: Option<String>,
}

pub fn clean_output(work_root: &str, conf_name: &str, local_only: bool) -> Result<GenCleanReport> {
    if !local_only {
        return Ok(GenCleanReport {
            note: Some("local_only=false (skip)".into()),
            ..Default::default()
        });
    }
    match load_wpgen_resolved(conf_name, work_root) {
        Ok(conf) => {
            if let Some(p) = conf.out_sink.resolve_file_path() {
                let existed = Path::new(&p).exists();
                let cleaned = if existed {
                    std::fs::remove_file(&p).is_ok()
                } else {
                    false
                };
                Ok(GenCleanReport {
                    path: Some(p),
                    existed,
                    cleaned,
                    note: None,
                })
            } else {
                Ok(GenCleanReport {
                    note: Some("output target is not a file sink".into()),
                    ..Default::default()
                })
            }
        }
        Err(_e) => Ok(GenCleanReport {
            note: Some(format!("config '{}' not found or invalid", conf_name)),
            ..Default::default()
        }),
    }
}

pub async fn sample_exec_direct_core(
    rule_root: &str,
    find_name: &str,
    prepared: (SampleGRA, SinkInstanceConf),
    rate_limit_rps: usize,
) -> RunResult<()> {
    let g = prepared.0.gen_conf.clone();
    run_sample_direct(rule_root, find_name, &g, &prepared.1, rate_limit_rps).await
}

pub async fn rule_exec_direct_core(
    stat_print: bool,
    rule_root: &str,
    prepared: (RuleGRA, SinkInstanceConf),
    rate_limit_rps: usize,
) -> RunResult<()> {
    let g = prepared.0.gen_conf.clone();
    let _ = stat_print;
    run_rule_direct(rule_root, &g, &prepared.1, rate_limit_rps).await
}
