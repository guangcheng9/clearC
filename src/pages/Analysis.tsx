import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { CleanupScanResult, formatBytes, scanCleanupRules } from "../lib/scan";
import { cancelTask, completeTask, failTask, requestTaskCancel, startTask } from "../lib/tasks";

type TaskProgressEvent = {
  task: string;
  current: number;
  total: number;
  label: string;
  status: string;
  scannedFiles?: number;
  scannedDirs?: number;
  scannedBytes?: number;
  skippedCount?: number;
  currentPath?: string;
};

export function Analysis() {
  const [results, setResults] = useState<CleanupScanResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [progress, setProgress] = useState<TaskProgressEvent | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listen<TaskProgressEvent>("clearc://task-progress", (event) => {
      if (!disposed && event.payload.task === "scan-cleanup-rules") {
        setProgress(event.payload);
      }
    }).then((handler) => {
      unlisten = handler;
    });

    const taskId = startTask("扫描清理规则路径", "analysis");
    scanCleanupRules()
      .then((result) => {
        setResults(result);
        completeTask(taskId, `扫描 ${result.length} 项`);
      })
      .catch((err) => {
        const message = String(err);
        setError(message);
        if (message.includes("cancelled")) {
          cancelTask(taskId, message);
        } else {
          failTask(taskId, message);
        }
      })
      .finally(() => setLoading(false));

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const totalBytes = results.reduce((sum, item) => sum + item.totalBytes, 0);
  const fileCount = results.reduce((sum, item) => sum + item.fileCount, 0);
  const skippedCount = results.reduce((sum, item) => sum + item.skippedCount, 0);
  const progressPercent = progress?.total ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>空间分析</h2>
        <p>当前展示清理规则命中的只读扫描结果。后续会扩展到大文件、大目录和重复文件检测。</p>
        {loading ? <p>扫描中... {progress?.label ?? ""}</p> : null}
        {loading ? (
          <div className="progress-track" aria-label="扫描进度">
            <div className="progress-fill" style={{ width: `${progressPercent}%` }} />
          </div>
        ) : null}
        {loading && progress ? (
          <p className="hint-text">
            文件 {progress.scannedFiles ?? 0} / 目录 {progress.scannedDirs ?? 0} / 跳过{" "}
            {progress.skippedCount ?? 0} / {formatBytes(progress.scannedBytes ?? 0)}
          </p>
        ) : null}
        {loading && progress?.currentPath ? <p className="hint-text">当前路径：{progress.currentPath}</p> : null}
        {loading && progress?.task ? (
          <button className="secondary-action" onClick={() => requestTaskCancel(progress.task)} type="button">
            取消扫描
          </button>
        ) : null}
        {error ? <p className="error-text">{error}</p> : null}
      </section>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>规则项</span>
          <strong>{results.length}</strong>
        </div>
        <div className="metric">
          <span>命中文件</span>
          <strong>{fileCount}</strong>
        </div>
        <div className="metric">
          <span>估算占用</span>
          <strong>{formatBytes(totalBytes)}</strong>
        </div>
        <div className="metric">
          <span>跳过项</span>
          <strong>{skippedCount}</strong>
        </div>
      </div>

      <section className="panel wide">
        <h2>规则扫描结果</h2>
        <ul className="placeholder-list">
          {results.map((result) => (
            <li key={result.id}>
              <span>
                {result.name}
                <small>
                  {result.paths
                    .map((path) => `${path.resolvedPath}${path.exists ? "" : " (不存在)"}`)
                    .join(", ")}
                </small>
              </span>
              <span className="tag">{formatBytes(result.totalBytes)}</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="panel">
        <h2>目录排行</h2>
        <p>下一阶段会把扫描结果扩展为系统盘目录排行，并增加进度事件。</p>
      </section>
      <section className="panel">
        <h2>安全边界</h2>
        <p>当前页面只读统计，不删除文件，不修改配置，不迁移目录。</p>
      </section>
    </div>
  );
}
