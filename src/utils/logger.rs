use chrono::Local;
use fern::{log_file, Dispatch};
use log::{LevelFilter, Level};
use std::{fs, path::Path};
use colored::Colorize; 

pub fn setup(log_dir: &str, filename: &str, is_remove_old: bool) -> Result<(), fern::InitError> {
    // 确保日志目录存在
    fs::create_dir_all(&log_dir)?;
    let mut log_filename = filename.to_string();
    // 获取当前日期
    let today = Local::now();
    if is_remove_old {
        log_filename = format!("app-{}.log", today.format("%Y-%m-%d"));
    }
    let log_file_path = Path::new(log_dir).join(log_filename);

    // 设置日志格式器
    let formatter =
        |out: fern::FormatCallback, message: &std::fmt::Arguments, record: &log::Record| {
            let line = format!(
                "{}[{}:{} {}],{},drg,{}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.target(),
                record.level(),
                message
            );
            let colored_message = match record.level() {
                Level::Error => line.red(),
                Level::Warn => line.yellow(),
                Level::Info => line.green(),
                Level::Debug => line.blue(),
                Level::Trace => line.white(),
            };
            
            out.finish(format_args!("{}", colored_message));
        };

    // 创建文件日志记录器
    let file_dispatch = Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Info)
        .chain(log_file(log_file_path)?);

    // 创建标准输出日志记录器
    let stdout_dispatch = Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Info)
        .chain(std::io::stdout());

    // 合并两个派发配置并应用
    let combined_config = Dispatch::new().chain(file_dispatch).chain(stdout_dispatch);
    combined_config.apply()?;

    // 删除旧的日志文件
    if is_remove_old {
        cleanup_old_logs(log_dir)?;
    }

    Ok(())
}

fn cleanup_old_logs(log_dir: &str) -> std::io::Result<()> {
    let mut entries = fs::read_dir(log_dir)?
        .filter_map(|res| res.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map(|s| s == "log").unwrap_or(false))
        .collect::<Vec<_>>();

    // 按修改时间排序，保留最新的三个文件
    entries.sort_by_key(|entry| {
        std::fs::metadata(entry.path())
            .and_then(|meta| meta.modified())
            .ok()
    });
    if entries.len() > 3 {
        for entry in &entries[..entries.len() - 3] {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}
