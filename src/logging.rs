//! 结构化文件日志:tracing 落盘 + 全局 panic/SIGSEGV 前兆钩子。
//!
//! 背景(2026-11-11 事故):fd 1/2 均为 /dev/null 时,tracing 与
//! `fatal runtime error: deadlock in SIGSEGV handler` 等致命输出全部无声丢弃,
//! 进程退化为单核 100% 自旋 16 小时而无任何痕迹。本模块保证致命错误至少落盘。

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// 事故现场的复刻条件之一是 stderr 指向 /dev/null;
/// 文件日志是独立通道,不受 fd 重定向影响。
fn log_file() -> &'static Mutex<Option<BufWriter<File>>> {
    static LOG: OnceLock<Mutex<Option<BufWriter<File>>>> = OnceLock::new();
    LOG.get_or_init(|| Mutex::new(None))
}

/// ~/.cache/ohmic/logs/ohmic.log(macOS 上对应 ~/Library/Caches 语义目录)。
/// 创建失败则返回 None,调用方降级为仅 stderr。
pub fn default_log_path() -> Option<PathBuf> {
    let dir = if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Caches/ohmic/logs"))
            .filter(|p| p.exists() || std::fs::create_dir_all(p).is_ok())
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|base| base.join("ohmic/logs"))
            .filter(|p| p.exists() || std::fs::create_dir_all(p).is_ok())
    }?;
    Some(dir.join("ohmic.log"))
}

fn open_writer(path: &PathBuf) -> Option<BufWriter<File>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()?;
    Some(BufWriter::new(file))
}

/// 初始化:tracing 写 stderr + 文件双通道;注册 panic 钩子把回溯写入文件。
/// 任何一步失败都静默降级——日志永远不能阻塞主功能。
pub fn init() {
    let path = default_log_path();
    if let Some(p) = &path {
        let w = open_writer(p);
        // 写入启动分隔行,便于事后按会话切分。
        if let Some(mut w) = w {
            let _ = writeln!(
                w,
                "===== ohmic start pid={} argv={:?} =====",
                std::process::id(),
                std::env::args().collect::<Vec<_>>()
            );
            let _ = w.flush();
            *log_file().lock().unwrap() = Some(w);
        }
    }

    // tracing:stderr(保留 CLI 场景)+ 文件(若有)。format 紧凑,无 ANSI。
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(|| FileWriter);
    let stderr_layer = tracing_subscriber::fmt::layer().with_ansi(false);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with(stderr_layer)
        .with(file_layer)
        .init();
    // panic 钩子:默认实现只写 stderr;这里补一份带回溯的文件记录。
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // payload 可能是 &str / String / 其他 Any,统一降级为 Debug。
        let payload: Box<dyn std::fmt::Debug> = Box::new(info.payload());
        file_write_all(&format!(
            "PANIC at {} thread={:?}\npayload: {:?}\nbacktrace:\n{}",
            chrono_like_now(),
            std::thread::current().name().unwrap_or("<unnamed>"),
            payload,
            std::backtrace::Backtrace::force_capture()
        ));
        default_hook(info);
    }));
}

struct FileWriter;

impl std::io::Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        file_write_buf(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn file_write_buf(buf: &[u8]) {
    if let Ok(mut guard) = log_file().lock()
        && let Some(w) = guard.as_mut()
    {
        let _ = w.write_all(buf);
        let _ = w.flush();
    }
}

fn file_write_all(s: &str) {
    file_write_buf(s.as_bytes());
}

/// 供致命路径(panic 钩子之外的看门狗等)直接落盘的公开入口。
pub fn write_fatal(msg: &str) {
    file_write_all(&format!(
        "FATAL at {} pid={} {msg}\n",
        chrono_like_now(),
        std::process::id()
    ));
}

/// 无 chrono 依赖的时间戳:秒精度 UTC,格式 "2026-11-12T03:45:12Z"。
fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 民用时间换算(Howard Hinnant 算法),避免引入时间库。
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}
