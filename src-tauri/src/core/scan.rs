use crate::core::paths::expand_path;
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathScanSummary {
    pub raw_path: String,
    pub resolved_path: String,
    pub exists: bool,
    pub accessible: bool,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_bytes: u64,
    pub skipped_count: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScanProgressSnapshot {
    pub current_path: String,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_bytes: u64,
    pub skipped_count: u64,
}

pub fn scan_rule_path(raw_path: &str) -> PathScanSummary {
    scan_rule_path_with_progress(raw_path, &mut || false, &mut |_| {})
        .unwrap_or_else(|summary| summary)
}

pub fn scan_rule_path_with_progress<ShouldCancel, OnProgress>(
    raw_path: &str,
    should_cancel: &mut ShouldCancel,
    on_progress: &mut OnProgress,
) -> Result<PathScanSummary, PathScanSummary>
where
    ShouldCancel: FnMut() -> bool,
    OnProgress: FnMut(ScanProgressSnapshot),
{
    let resolved = expand_path(raw_path);
    let resolved_path = resolved.display().to_string();

    if !resolved.exists() {
        return Ok(PathScanSummary {
            raw_path: raw_path.to_string(),
            resolved_path,
            exists: false,
            accessible: false,
            file_count: 0,
            dir_count: 0,
            total_bytes: 0,
            skipped_count: 0,
            error: None,
        });
    }

    let mut summary = PathScanSummary {
        raw_path: raw_path.to_string(),
        resolved_path,
        exists: true,
        accessible: true,
        file_count: 0,
        dir_count: 0,
        total_bytes: 0,
        skipped_count: 0,
        error: None,
    };

    let mut visited_count = 0u64;
    let cancelled = scan_path_with_progress(
        &resolved,
        &mut summary,
        should_cancel,
        on_progress,
        &mut visited_count,
    );
    emit_scan_progress(&resolved, &summary, on_progress);

    if cancelled {
        summary.error = Some("scan cancelled".to_string());
        Err(summary)
    } else {
        Ok(summary)
    }
}

fn scan_path(path: &Path, summary: &mut PathScanSummary) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            summary.accessible = false;
            summary.skipped_count += 1;
            summary.error.get_or_insert_with(|| err.to_string());
            return;
        }
    };

    if metadata.is_file() {
        summary.file_count += 1;
        summary.total_bytes = summary.total_bytes.saturating_add(metadata.len());
        return;
    }

    if !metadata.is_dir() {
        return;
    }

    summary.dir_count += 1;
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            summary.accessible = false;
            summary.skipped_count += 1;
            summary.error.get_or_insert_with(|| err.to_string());
            return;
        }
    };

    for entry in entries {
        match entry {
            Ok(entry) => scan_path(&entry.path(), summary),
            Err(err) => {
                summary.skipped_count += 1;
                summary.error.get_or_insert_with(|| err.to_string());
            }
        }
    }
}

fn scan_path_with_progress<ShouldCancel, OnProgress>(
    path: &Path,
    summary: &mut PathScanSummary,
    should_cancel: &mut ShouldCancel,
    on_progress: &mut OnProgress,
    visited_count: &mut u64,
) -> bool
where
    ShouldCancel: FnMut() -> bool,
    OnProgress: FnMut(ScanProgressSnapshot),
{
    if should_cancel() {
        return true;
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            summary.accessible = false;
            summary.skipped_count += 1;
            summary.error.get_or_insert_with(|| err.to_string());
            return false;
        }
    };

    *visited_count = visited_count.saturating_add(1);

    if metadata.is_file() {
        summary.file_count += 1;
        summary.total_bytes = summary.total_bytes.saturating_add(metadata.len());
        if *visited_count % 64 == 0 {
            emit_scan_progress(path, summary, on_progress);
        }
        return false;
    }

    if !metadata.is_dir() {
        return false;
    }

    summary.dir_count += 1;
    if *visited_count % 64 == 0 {
        emit_scan_progress(path, summary, on_progress);
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            summary.accessible = false;
            summary.skipped_count += 1;
            summary.error.get_or_insert_with(|| err.to_string());
            return false;
        }
    };

    for entry in entries {
        match entry {
            Ok(entry) => {
                if scan_path_with_progress(
                    &entry.path(),
                    summary,
                    should_cancel,
                    on_progress,
                    visited_count,
                ) {
                    return true;
                }
            }
            Err(err) => {
                summary.skipped_count += 1;
                summary.error.get_or_insert_with(|| err.to_string());
            }
        }
    }

    false
}

fn emit_scan_progress<OnProgress>(
    path: &Path,
    summary: &PathScanSummary,
    on_progress: &mut OnProgress,
) where
    OnProgress: FnMut(ScanProgressSnapshot),
{
    on_progress(ScanProgressSnapshot {
        current_path: path.display().to_string(),
        file_count: summary.file_count,
        dir_count: summary.dir_count,
        total_bytes: summary.total_bytes,
        skipped_count: summary.skipped_count,
    });
}

#[cfg(windows)]
pub fn disk_space(path: &str) -> Result<(u64, u64), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    let mut free_available = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;

    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_available,
            &mut total,
            &mut total_free,
        )
    };

    if ok == 0 {
        return Err(format!("failed to read disk space for {}", path));
    }

    Ok((total, total_free))
}

#[cfg(not(windows))]
pub fn disk_space(_path: &str) -> Result<(u64, u64), String> {
    Ok((0, 0))
}
