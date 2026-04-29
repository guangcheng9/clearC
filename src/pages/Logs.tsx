import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { rollbackDevSpaceEnvMigration, rollbackDevSpaceJunctionMigration } from "../lib/devspace";
import { rollbackUserFolderMigration } from "../lib/migration";
import {
  exportOperationFailures,
  FailureExportResult,
  getLogSummary,
  LogSummary,
  openFailureExportFolder,
  rollbackQuarantineCleanup,
  RollbackResult,
} from "../lib/logs";
import { clearCompletedTasks, readTasks, requestTaskCancel, TaskRecord } from "../lib/tasks";

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
  currentFilePath?: string;
  currentFileBytes?: number;
  currentFileTotalBytes?: number;
};

export function Logs() {
  const [summary, setSummary] = useState<LogSummary | null>(null);
  const [rollbackResult, setRollbackResult] = useState<RollbackResult | null>(null);
  const [exportResult, setExportResult] = useState<FailureExportResult | null>(null);
  const [tasks, setTasks] = useState<TaskRecord[]>([]);
  const [progress, setProgress] = useState<TaskProgressEvent | null>(null);
  const [rollingBackId, setRollingBackId] = useState("");
  const [exportingId, setExportingId] = useState("");
  const [openingExportFolder, setOpeningExportFolder] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listen<TaskProgressEvent>("clearc://task-progress", (event) => {
      if (!disposed && event.payload.task === "migration-rollback") {
        setProgress(event.payload);
      }
    }).then((handler) => {
      unlisten = handler;
    });

    refreshLogs();
    refreshTasks();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  function refreshLogs() {
    getLogSummary()
      .then(setSummary)
      .catch((err) => setError(String(err)));
  }

  function refreshTasks() {
    setTasks(readTasks());
  }

  function clearFinishedTasks() {
    clearCompletedTasks();
    refreshTasks();
  }

  function rollback(operationId: string, operationType: string) {
    const isMigrationRollback = operationType === "user-folder-migration";
    const isDevSpaceEnvRollback = operationType === "devspace-env-migration";
    const isDevSpaceJunctionRollback = operationType === "devspace-junction-migration";
    const accepted = window.confirm(
      isMigrationRollback
        ? "确认回滚该用户目录迁移？回滚会尝试恢复文件位置并恢复 User Shell Folders。"
        : isDevSpaceEnvRollback
          ? "确认回滚该开发工具环境变量迁移？回滚会恢复或删除对应用户级环境变量。"
          : isDevSpaceJunctionRollback
            ? "确认回滚该 Junction 迁移？回滚会删除原路径 Junction 并尝试恢复文件。"
        : "确认回滚该隔离清理？回滚会尝试把隔离项恢复到原路径。"
    );
    if (!accepted) {
      return;
    }

    setRollingBackId(operationId);
    setError("");
    const rollbackTask =
      isMigrationRollback
        ? rollbackUserFolderMigration(operationId)
        : isDevSpaceEnvRollback
          ? rollbackDevSpaceEnvMigration(operationId)
          : isDevSpaceJunctionRollback
            ? rollbackDevSpaceJunctionMigration(operationId)
        : rollbackQuarantineCleanup(operationId);

    rollbackTask
      .then((result) => {
        setRollbackResult({
          id: result.id,
          rollbackOf: "rollbackOf" in result ? result.rollbackOf : operationId,
          status: result.status,
          restoredCount: "restoredCount" in result ? result.restoredCount : 0,
          skippedCount: result.skippedCount,
          failures: result.failures,
        });
        refreshLogs();
      })
      .catch((err) => setError(String(err)))
      .finally(() => setRollingBackId(""));
  }

  function exportFailures(operationId: string) {
    setExportingId(operationId);
    setError("");
    exportOperationFailures(operationId)
      .then(setExportResult)
      .catch((err) => setError(String(err)))
      .finally(() => setExportingId(""));
  }

  function openExports() {
    setOpeningExportFolder(true);
    setError("");
    openFailureExportFolder()
      .catch((err) => setError(String(err)))
      .finally(() => setOpeningExportFolder(false));
  }

  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>任务状态与日志</h2>
        <p>这里汇总最近任务、计划草稿、失败项和回滚入口。失败项可导出到本地 CSV。</p>
        {rollingBackId ? <p className="busy-text">正在执行回滚，请勿重复点击。</p> : null}
        {rollingBackId && progress?.task === "migration-rollback" ? (
          <p className="busy-text">{progress.label}</p>
        ) : null}
        {rollingBackId && progress?.task === "migration-rollback" ? (
          <div className="progress-track" aria-label="迁移回滚进度">
            <div
              className="progress-fill"
              style={{
                width: `${progress.total ? Math.round((progress.current / progress.total) * 100) : 0}%`,
              }}
            />
          </div>
        ) : null}
        {rollingBackId && progress?.task === "migration-rollback" ? (
          <p className="hint-text">
            已处理 {progress.processedItems ?? progress.current}/{progress.total} 项
            {typeof progress.movedCount === "number" ? ` / 已恢复 ${progress.movedCount}` : ""}
            {typeof progress.skippedCount === "number" ? ` / 跳过 ${progress.skippedCount}` : ""}
            {typeof progress.failureCount === "number" ? ` / 失败 ${progress.failureCount}` : ""}
          </p>
        ) : null}
        {rollingBackId && progress?.task === "migration-rollback" && progress.currentFilePath ? (
          <p className="hint-text">
            当前文件 {progress.currentFileBytes ?? 0}/{progress.currentFileTotalBytes ?? 0} 字节：
            {progress.currentFilePath}
          </p>
        ) : null}
        {rollingBackId && progress?.task === "migration-rollback" ? (
          <button className="secondary-action" onClick={() => requestTaskCancel(progress.task)} type="button">
            取消回滚
          </button>
        ) : null}
        {exportingId ? <p className="busy-text">正在导出失败项。</p> : null}
        {error ? <p className="error-text">{error}</p> : null}
      </section>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>操作记录</span>
          <strong>{summary?.totalOperations ?? 0}</strong>
        </div>
        <div className="metric">
          <span>计划草稿</span>
          <strong>{summary?.plannedOperations ?? 0}</strong>
        </div>
        <div className="metric">
          <span>可回滚</span>
          <strong>{summary?.rollbackableOperations ?? 0}</strong>
        </div>
        <div className="metric">
          <span>异常任务</span>
          <strong>{summary?.failedOperations ?? 0}</strong>
        </div>
      </div>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>日志状态</span>
          <strong>{summary ? "就绪" : "读取中"}</strong>
        </div>
        <div className="metric">
          <span>最近任务</span>
          <strong>{summary?.recent.length ?? 0}</strong>
        </div>
        <div className="metric">
          <span>执行中</span>
          <strong>
            {tasks.filter((task) => task.status === "running").length + (rollingBackId || exportingId ? 1 : 0)}
          </strong>
        </div>
        <div className="metric">
          <span>取消能力</span>
          <strong>可用</strong>
        </div>
      </div>

      <section className="panel wide">
        <h2>本地任务队列</h2>
        <button className="secondary-action" onClick={refreshTasks} type="button">
          刷新任务状态
        </button>
        <button className="secondary-action" onClick={clearFinishedTasks} type="button">
          清理已完成任务
        </button>
        <ul className="placeholder-list">
          {tasks.map((task) => (
            <li key={task.id}>
              <span>
                {task.label}
                <small>
                  {task.source} / {task.status}
                  {task.detail ? ` / ${task.detail}` : ""}
                </small>
              </span>
              <span className="tag">{task.finishedAt ? "done" : "running"}</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="panel wide">
        <h2>最近任务</h2>
        <ul className="placeholder-list">
          {(summary?.recent ?? []).map((entry) => (
            <li key={entry.id}>
              <span>
                {entry.summary}
                <small>
                  {entry.operationType} / {entry.status} / 失败项 {entry.failureCount}
                </small>
              </span>
              {entry.failureCount ? (
                <button
                  className="inline-action"
                  disabled={exportingId === entry.id}
                  onClick={() => exportFailures(entry.id)}
                  type="button"
                >
                  {exportingId === entry.id ? "导出中" : "导出失败项"}
                </button>
              ) : null}
              {entry.canRollback ? (
                <button
                  className="inline-action"
                  disabled={rollingBackId === entry.id}
                  onClick={() => rollback(entry.id, entry.operationType)}
                  type="button"
                >
                  {rollingBackId === entry.id
                    ? "回滚中"
                    : entry.operationType === "user-folder-migration"
                      ? "回滚迁移"
                      : entry.operationType === "devspace-env-migration"
                        ? "回滚环境变量"
                        : entry.operationType === "devspace-junction-migration"
                          ? "回滚 Junction"
                      : "回滚清理"}
                </button>
              ) : (
                <span className="tag">{entry.rollbackable ? "done" : "record"}</span>
              )}
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>回滚策略</h2>
        <p>涉及配置、环境变量、Junction 或 Shell Folder 的修改都必须保存修改前状态。</p>
        {rollbackResult ? (
          <p>
            最近回滚恢复 {rollbackResult.restoredCount} 项，跳过 {rollbackResult.skippedCount} 项。
          </p>
        ) : null}
      </section>
      <section className="panel">
        <h2>导出结果</h2>
        {exportResult ? (
          <>
            <p>
              已导出 {exportResult.exportedCount} 条失败项：{exportResult.path}
            </p>
            <button className="secondary-action" disabled={openingExportFolder} onClick={openExports} type="button">
              {openingExportFolder ? "打开中..." : "打开导出目录"}
            </button>
          </>
        ) : (
          <>
            <p>有失败项的任务会显示导出按钮，导出文件位于 `.clearc/exports`。</p>
            <button className="secondary-action" disabled={openingExportFolder} onClick={openExports} type="button">
              {openingExportFolder ? "打开中..." : "打开导出目录"}
            </button>
          </>
        )}
      </section>
    </div>
  );
}
