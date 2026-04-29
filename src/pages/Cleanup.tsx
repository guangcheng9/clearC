import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import {
  CleanupPlanDraft,
  CleanupPreview,
  QuarantineCleanupResult,
  createCleanupPlanDraft,
  executeTempQuarantineCleanup,
  getCleanupPreview,
} from "../lib/cleanup";
import { CleanupRule, getRuleCatalog } from "../lib/rules";
import { formatBytes } from "../lib/scan";
import { cancelTask, completeTask, failTask, requestTaskCancel, startTask } from "../lib/tasks";

type TaskProgressEvent = {
  task: string;
  current: number;
  total: number;
  label: string;
  status: string;
  processedItems?: number;
  movedCount?: number;
  skippedCount?: number;
  processedBytes?: number;
  failureCount?: number;
  scannedFiles?: number;
  scannedDirs?: number;
  scannedBytes?: number;
  currentPath?: string;
  currentFilePath?: string;
  currentFileBytes?: number;
  currentFileTotalBytes?: number;
};

export function Cleanup() {
  const [rules, setRules] = useState<CleanupRule[]>([]);
  const [preview, setPreview] = useState<CleanupPreview | null>(null);
  const [draft, setDraft] = useState<CleanupPlanDraft | null>(null);
  const [cleanupResult, setCleanupResult] = useState<QuarantineCleanupResult | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [creatingDraft, setCreatingDraft] = useState(false);
  const [executingCleanup, setExecutingCleanup] = useState(false);
  const [progress, setProgress] = useState<TaskProgressEvent | null>(null);
  const [error, setError] = useState("");
  const busy = loadingPreview || creatingDraft || executingCleanup;

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listen<TaskProgressEvent>("clearc://task-progress", (event) => {
      if (
        !disposed &&
        (event.payload.task === "cleanup-preview" || event.payload.task === "cleanup-quarantine")
      ) {
        setProgress(event.payload);
      }
    }).then((handler) => {
      unlisten = handler;
    });

    getRuleCatalog()
      .then((catalog) => setRules(catalog.cleanup))
      .catch((err) => setError(String(err)));

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  function refreshPreview() {
    const taskId = startTask("生成清理预览", "cleanup");
    setLoadingPreview(true);
    setError("");
    setDraft(null);
    setCleanupResult(null);
    getCleanupPreview()
      .then((result) => {
        setPreview(result);
        completeTask(taskId, `预计释放 ${formatBytes(result.estimatedBytes)}`);
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
      .finally(() => setLoadingPreview(false));
  }

  function createDraft() {
    const taskId = startTask("记录清理确认草稿", "cleanup");
    setCreatingDraft(true);
    setError("");
    createCleanupPlanDraft()
      .then((result) => {
        setDraft(result);
        completeTask(taskId, result.status);
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
      .finally(() => setCreatingDraft(false));
  }

  function executeQuarantineCleanup() {
    const taskId = startTask("隔离清理用户临时文件", "cleanup");
    setExecutingCleanup(true);
    setError("");
    executeTempQuarantineCleanup()
      .then((result) => {
        setCleanupResult(result);
        completeTask(taskId, `移动 ${result.movedCount} 项，跳过 ${result.skippedCount} 项`);
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
      .finally(() => setExecutingCleanup(false));
  }

  const progressPercent = progress?.total ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div className="page-grid">
      <section className="panel wide" aria-busy={busy}>
        <h2>安全清理</h2>
        <p>当前阶段只生成清理预览和确认模型，不执行删除。后续接入真实清理时会复用这套预览结果。</p>
        {busy ? (
          <p className="busy-text">
            正在处理，请稍候。窗口可以继续响应。{progress?.label ?? ""}
          </p>
        ) : null}
        {busy ? (
          <div className="progress-track" aria-label="清理任务进度">
            <div className="progress-fill" style={{ width: `${progressPercent}%` }} />
          </div>
        ) : null}
        {busy && progress ? (
          <p className="hint-text">
            已处理 {progress.processedItems ?? progress.current}/{progress.total} 项
            {typeof progress.movedCount === "number" ? ` / 已移动 ${progress.movedCount}` : ""}
            {typeof progress.skippedCount === "number" ? ` / 跳过 ${progress.skippedCount}` : ""}
            {typeof progress.failureCount === "number" ? ` / 失败 ${progress.failureCount}` : ""}
            {typeof progress.processedBytes === "number" ? ` / ${formatBytes(progress.processedBytes)}` : ""}
          </p>
        ) : null}
        {busy && typeof progress?.scannedFiles === "number" ? (
          <p className="hint-text">
            扫描文件 {progress.scannedFiles} / 目录 {progress.scannedDirs ?? 0} / 跳过{" "}
            {progress.skippedCount ?? 0} / {formatBytes(progress.scannedBytes ?? 0)}
          </p>
        ) : null}
        {busy && progress?.currentPath ? <p className="hint-text">当前路径：{progress.currentPath}</p> : null}
        {busy && progress?.currentFilePath ? (
          <p className="hint-text">
            当前文件 {formatBytes(progress.currentFileBytes ?? 0)}/
            {formatBytes(progress.currentFileTotalBytes ?? 0)}：{progress.currentFilePath}
          </p>
        ) : null}
        {busy && progress?.task ? (
          <button className="secondary-action" onClick={() => requestTaskCancel(progress.task)} type="button">
            取消当前任务
          </button>
        ) : null}
        <button className="primary-action" disabled={busy} onClick={refreshPreview} type="button">
          {loadingPreview ? "生成中..." : "生成清理预览"}
        </button>
        <button
          className="secondary-action"
          disabled={!preview || busy}
          onClick={createDraft}
          type="button"
        >
          {creatingDraft ? "记录中..." : "记录确认草稿"}
        </button>
        <button
          className="danger-action"
          disabled={!draft || busy}
          onClick={executeQuarantineCleanup}
          type="button"
        >
          {executingCleanup ? "隔离中..." : "移动临时文件到隔离区"}
        </button>
        {!preview ? <p className="hint-text">先生成清理预览后，才能记录草稿。</p> : null}
        {preview && !draft ? <p className="hint-text">先记录确认草稿后，才能执行隔离移动。</p> : null}
      </section>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>预计释放</span>
          <strong>{formatBytes(preview?.estimatedBytes ?? 0)}</strong>
        </div>
        <div className="metric">
          <span>文件数量</span>
          <strong>{preview?.fileCount ?? 0}</strong>
        </div>
        <div className="metric">
          <span>跳过项</span>
          <strong>{preview?.skippedCount ?? 0}</strong>
        </div>
        <div className="metric">
          <span>执行状态</span>
          <strong>{preview?.executable ? "可执行" : "仅预览"}</strong>
        </div>
      </div>

      <section className="panel">
        <h2>规则文件清理项</h2>
        {error ? <p className="error-text">{error}</p> : null}
        <ul className="placeholder-list">
          {rules.map((rule) => (
            <li key={rule.id}>
              <span>
                {rule.name}
                <small>{rule.paths.join(", ")}</small>
              </span>
              <span className="tag">{rule.risk}</span>
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>确认模型</h2>
        <p>
          {preview?.requiresConfirmation
            ? "清理动作必须二次确认。当前版本尚未开放执行按钮。"
            : "当前预览无需确认。"}
        </p>
        {draft ? (
          <p>
            已记录草稿 <code>{draft.id}</code>，状态为 {draft.status}。
          </p>
        ) : null}
        {cleanupResult ? (
          <p>
            已移动 {cleanupResult.movedCount} 项到隔离区，跳过 {cleanupResult.skippedCount} 项。
          </p>
        ) : null}
      </section>

      {cleanupResult ? (
        <section className="panel wide">
          <h2>隔离结果</h2>
          <ul className="placeholder-list">
            <li>
              <span>
                隔离目录
                <small>{cleanupResult.quarantinePath}</small>
              </span>
              <span className="tag">{formatBytes(cleanupResult.movedBytes)}</span>
            </li>
            {cleanupResult.failures.map((failure) => (
              <li key={`${failure.path}-${failure.reason}`}>
                <span>
                  {failure.path}
                  <small>{failure.reason}</small>
                </span>
                <span className="tag">skipped</span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>预览明细</h2>
        <ul className="placeholder-list">
          {(preview?.items ?? []).map((item) => (
            <li key={item.id}>
              <span>
                {item.name}
                <small>
                  {item.paths
                    .map((path) => `${path.resolvedPath}${path.exists ? "" : " (不存在)"}`)
                    .join(", ")}
                </small>
              </span>
              <span className="tag">{formatBytes(item.estimatedBytes)}</span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
