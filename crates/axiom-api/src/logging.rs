use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing_appender::non_blocking;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub tracing_enabled: bool,
    pub otlp_endpoint: Option<String>,
    pub log_file: Option<PathBuf>,
    pub rotation_size_mb: Option<u64>,
    pub max_log_files: Option<usize>,
}

#[derive(Clone, Copy)]
pub enum LogFormat {
    Json,
    Text,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            tracing_enabled: false,
            otlp_endpoint: None,
            log_file: None,
            rotation_size_mb: Some(10),
            max_log_files: Some(5),
        }
    }
}

struct RollingFileWriter {
    path: PathBuf,
    rotation_size: u64,
    max_files: usize,
    current_size: Arc<AtomicUsize>,
    file: Arc<std::sync::Mutex<File>>,
}

impl RollingFileWriter {
    fn new(path: PathBuf, rotation_size: u64, max_files: usize) -> io::Result<Self> {
        if let Some(parent_dir) = path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let file = OpenOptions::new().create(true).append(true).truncate(false).open(&path)?;

        let current_size = file.metadata()?.len() as usize;

        Ok(Self {
            path,
            rotation_size,
            max_files,
            current_size: Arc::new(AtomicUsize::new(current_size)),
            file: Arc::new(std::sync::Mutex::new(file)),
        })
    }

    fn rotate(&self) -> io::Result<()> {
        let mut file = self.file.lock().map_err(|_| io::Error::other("mutex lock failed"))?;

        for i in (1..self.max_files).rev() {
            let old_path = self.path.with_extension(format!("log.{}", i));
            let new_path = self.path.with_extension(format!("log.{}", i + 1));
            if old_path.exists() {
                fs::rename(&old_path, &new_path)?;
            }
        }

        let backup_path = self.path.with_extension("log.1");
        if self.path.exists() {
            fs::rename(&self.path, &backup_path)?;
        }

        *file = OpenOptions::new().create(true).write(true).truncate(true).open(&self.path)?;

        self.current_size.store(0, Ordering::Relaxed);
        Ok(())
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = {
            let mut file = self.file.lock().map_err(|_| io::Error::other("mutex lock failed"))?;
            file.write(buf)?
        };

        let new_size = self.current_size.fetch_add(written, Ordering::Relaxed) + written;
        if new_size as u64 >= self.rotation_size {
            let _ = self.rotate();
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file = self.file.lock().map_err(|_| io::Error::other("mutex lock failed"))?;
        file.flush()
    }
}

pub fn init_logging(config: LoggingConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    if config.tracing_enabled {
        let endpoint =
            config.otlp_endpoint.unwrap_or_else(|| "http://localhost:4318/v1/traces".to_string());
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .expect("failed to create OTLP exporter");

        let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();

        global::set_tracer_provider(provider);

        if let Some(log_file) = &config.log_file {
            let rotation_size_mb = config.rotation_size_mb.unwrap_or(10);
            let max_files = config.max_log_files.unwrap_or(5);

            let file_writer =
                RollingFileWriter::new(log_file.clone(), rotation_size_mb * 1024 * 1024, max_files)
                    .expect("failed to create rolling file appender");

            let (non_blocking, _guard) = non_blocking(file_writer);

            match config.format {
                LogFormat::Json => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_opentelemetry::layer())
                        .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking))
                        .try_init();
                }
                LogFormat::Text => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_opentelemetry::layer())
                        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
                        .try_init();
                }
            }
        } else {
            match config.format {
                LogFormat::Json => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_opentelemetry::layer())
                        .with(tracing_subscriber::fmt::layer().json())
                        .try_init();
                }
                LogFormat::Text => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_opentelemetry::layer())
                        .with(tracing_subscriber::fmt::layer())
                        .try_init();
                }
            }
        }
    } else {
        if let Some(log_file) = &config.log_file {
            let rotation_size_mb = config.rotation_size_mb.unwrap_or(10);
            let max_files = config.max_log_files.unwrap_or(5);

            let file_writer =
                RollingFileWriter::new(log_file.clone(), rotation_size_mb * 1024 * 1024, max_files)
                    .expect("failed to create rolling file appender");

            let (non_blocking, _guard) = non_blocking(file_writer);

            match config.format {
                LogFormat::Json => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_subscriber::fmt::layer().json().with_writer(non_blocking))
                        .try_init();
                }
                LogFormat::Text => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
                        .try_init();
                }
            }
        } else {
            match config.format {
                LogFormat::Json => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_subscriber::fmt::layer().json())
                        .try_init();
                }
                LogFormat::Text => {
                    let _ = Registry::default()
                        .with(filter)
                        .with(tracing_subscriber::fmt::layer())
                        .try_init();
                }
            }
        }
    }
}
