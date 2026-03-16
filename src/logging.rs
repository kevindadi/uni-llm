//! tracing 日志初始化.

use std::io;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::LoggingConfig;

/// 初始化日志,根据配置设置 level、是否输出到文件.
pub fn init_logging(config: &LoggingConfig) -> Result<(), io::Error> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    let base_fmt = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_span_events(FmtSpan::CLOSE);

    if config.log_to_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file)?;
        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        std::mem::forget(guard);
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_target(true)
            .with_ansi(false);
        tracing_subscriber::registry()
            .with(filter)
            .with(base_fmt)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(base_fmt)
            .init();
    }

    Ok(())
}
