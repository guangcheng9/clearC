import { useEffect, useState } from "react";
import { rollbackUserFolderMigration } from "../lib/migration";
import { getLogSummary, LogSummary, rollbackQuarantineCleanup, RollbackResult } from "../lib/logs";

export function Logs() {
  const [summary, setSummary] = useState<LogSummary | null>(null);
  const [rollbackResult, setRollbackResult] = useState<RollbackResult | null>(null);
  const [rollingBackId, setRollingBackId] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    refreshLogs();
  }, []);

  function refreshLogs() {
    getLogSummary()
      .then(setSummary)
      .catch((err) => setError(String(err)));
  }

  function rollback(operationId: string, operationType: string) {
    const isMigrationRollback = operationType === "user-folder-migration";
    const accepted = window.confirm(
      isMigrationRollback
        ? "确认回滚该用户目录迁移？回滚会尝试恢复文件位置并恢复 User Shell Folders。"
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

  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>日志与回滚</h2>
        <p>所有清理和迁移动作都会记录操作时间、路径、大小、成功项、失败项和回滚信息。</p>
        {rollingBackId ? <p className="busy-text">正在执行回滚，请勿重复点击。</p> : null}
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
          <span>日志状态</span>
          <strong>{summary ? "就绪" : "读取中"}</strong>
        </div>
      </div>

      <section className="panel">
        <h2>操作日志</h2>
        <ul className="placeholder-list">
          {(summary?.recent ?? []).map((entry) => (
            <li key={entry.id}>
              <span>
                {entry.summary}
                <small>
                  {entry.operationType} / {entry.status}
                </small>
              </span>
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
    </div>
  );
}
