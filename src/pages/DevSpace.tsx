import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import {
  createDevSpaceEnvMigrationPlan,
  createDevSpaceJunctionPlan,
  DevSpaceEnvMigrationPlan,
  DevSpaceEnvMigrationResult,
  DevSpaceJunctionMigrationResult,
  DevSpaceJunctionPlan,
  DevSpaceScanResult,
  executeDevSpaceJunctionMigration,
  executeDevSpaceEnvMigration,
  scanDevSpaceTargets,
} from "../lib/devspace";
import { DevSpaceRule, getRuleCatalog } from "../lib/rules";
import { formatBytes } from "../lib/scan";
import { cancelTask, completeTask, failTask, requestTaskCancel, startTask } from "../lib/tasks";

type TaskProgressEvent = {
  task: string;
  current: number;
  total: number;
  label: string;
  status: string;
};

export function DevSpace() {
  const [rules, setRules] = useState<DevSpaceRule[]>([]);
  const [results, setResults] = useState<DevSpaceScanResult[]>([]);
  const [targetRoot, setTargetRoot] = useState("D:\\DevData");
  const [plans, setPlans] = useState<Record<string, DevSpaceEnvMigrationPlan>>({});
  const [junctionPlans, setJunctionPlans] = useState<Record<string, DevSpaceJunctionPlan>>({});
  const [migrationResults, setMigrationResults] = useState<Record<string, DevSpaceEnvMigrationResult>>({});
  const [junctionResults, setJunctionResults] = useState<Record<string, DevSpaceJunctionMigrationResult>>({});
  const [confirmations, setConfirmations] = useState<Record<string, string>>({});
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<TaskProgressEvent | null>(null);
  const [planningId, setPlanningId] = useState("");
  const [executingId, setExecutingId] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listen<TaskProgressEvent>("clearc://task-progress", (event) => {
      if (!disposed && event.payload.task === "scan-devspace-targets") {
        setProgress(event.payload);
      }
    }).then((handler) => {
      unlisten = handler;
    });

    getRuleCatalog()
      .then((catalog) => setRules(catalog.devspace))
      .catch((err) => setError(String(err)));

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  function runScan() {
    const taskId = startTask("扫描开发工具目录", "devspace");
    setScanning(true);
    setError("");
    scanDevSpaceTargets()
      .then((result) => {
        setResults(result);
        completeTask(taskId, `命中 ${result.filter((item) => item.exists).length} 项`);
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
      .finally(() => setScanning(false));
  }

  function createEnvPlan(targetId: string) {
    const taskId = startTask("生成开发工具环境变量迁移计划", "devspace");
    setPlanningId(targetId);
    setError("");
    createDevSpaceEnvMigrationPlan(targetId, targetRoot)
      .then((plan) =>
        setPlans((current) => ({
          ...current,
          [targetId]: plan,
        }))
      )
      .then(() => completeTask(taskId, "planned"))
      .catch((err) => {
        const message = String(err);
        setError(message);
        failTask(taskId, message);
      })
      .finally(() => setPlanningId(""));
  }

  function executeEnvMigration(targetId: string) {
    const plan = plans[targetId];
    const confirmation = confirmations[targetId] ?? "";
    if (!plan?.canExecute || confirmation !== "MIGRATE_DEVSPACE_ENV") {
      setError("执行开发工具环境变量迁移前，需要先生成可执行计划，并输入确认短语 MIGRATE_DEVSPACE_ENV。");
      return;
    }

    const accepted = window.confirm(
      `确认修改 ${plan.name} 的用户级环境变量？\n\n${plan.variables
        .map((variable) => `${variable.name}: ${variable.originalValue ?? "(未设置)"} -> ${variable.newValue}`)
        .join("\n")}`
    );
    if (!accepted) {
      return;
    }

    setExecutingId(targetId);
    setError("");
    const taskId = startTask(`执行开发工具环境变量迁移：${plan.name}`, "devspace");
    executeDevSpaceEnvMigration(targetId, targetRoot, confirmation)
      .then((result) =>
        setMigrationResults((current) => ({
          ...current,
          [targetId]: result,
        }))
      )
      .then(() => completeTask(taskId, "completed"))
      .catch((err) => {
        const message = String(err);
        setError(message);
        failTask(taskId, message);
      })
      .finally(() => setExecutingId(""));
  }

  function createJunctionPlan(targetId: string) {
    const taskId = startTask("生成 Junction 迁移预案", "devspace");
    setPlanningId(targetId);
    setError("");
    createDevSpaceJunctionPlan(targetId, targetRoot)
      .then((plan) =>
        setJunctionPlans((current) => ({
          ...current,
          [targetId]: plan,
        }))
      )
      .then(() => completeTask(taskId, "plan-only"))
      .catch((err) => {
        const message = String(err);
        setError(message);
        failTask(taskId, message);
      })
      .finally(() => setPlanningId(""));
  }

  function executeJunctionMigration(targetId: string) {
    const plan = junctionPlans[targetId];
    const confirmation = confirmations[`junction-${targetId}`] ?? "";
    if (!plan?.canExecute || confirmation !== "CREATE_DEVSPACE_JUNCTION") {
      setError("执行 Junction 迁移前，需要先生成可执行预案，并输入确认短语 CREATE_DEVSPACE_JUNCTION。");
      return;
    }

    const accepted = window.confirm(
      `确认创建 ${plan.name} 的 Junction？\n\n${plan.items
        .map((item) => `${item.sourcePath} -> ${item.targetPath}`)
        .join("\n")}\n\n该操作会移动目录并在原路径创建 Junction。`
    );
    if (!accepted) {
      return;
    }

    setExecutingId(targetId);
    setError("");
    const taskId = startTask(`执行 Junction 迁移：${plan.name}`, "devspace");
    executeDevSpaceJunctionMigration(targetId, targetRoot, confirmation)
      .then((result) => {
        setJunctionResults((current) => ({
          ...current,
          [targetId]: result,
        }));
        completeTask(taskId, `迁移 ${result.movedCount} 项，跳过 ${result.skippedCount} 项`);
      })
      .catch((err) => {
        const message = String(err);
        setError(message);
        failTask(taskId, message);
      })
      .finally(() => setExecutingId(""));
  }

  const existingResults = results.filter((result) => result.exists);
  const totalBytes = results.reduce((sum, result) => sum + result.totalBytes, 0);
  const skippedCount = results.reduce((sum, result) => sum + result.skippedCount, 0);
  const progressPercent = progress?.total ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div className="page-grid">
      <section className="panel wide" aria-busy={scanning}>
        <h2>开发工具空间管理</h2>
        <p>当前支持只读扫描、环境变量迁移，以及 Junction 迁移预案；Junction 预案只检测和规划，不创建链接。</p>
        {scanning ? (
          <p className="busy-text">
            正在扫描开发工具目录，路径较大时可能需要一些时间。{progress?.label ?? ""}
          </p>
        ) : null}
        {scanning ? (
          <div className="progress-track" aria-label="开发工具扫描进度">
            <div className="progress-fill" style={{ width: `${progressPercent}%` }} />
          </div>
        ) : null}
        {scanning && progress?.task ? (
          <button className="secondary-action" onClick={() => requestTaskCancel(progress.task)} type="button">
            取消扫描
          </button>
        ) : null}
        {planningId ? <p className="busy-text">正在生成环境变量迁移计划。</p> : null}
        {executingId ? <p className="busy-text">正在修改用户级环境变量，请勿重复点击。</p> : null}
        <button className="primary-action" disabled={scanning} onClick={runScan} type="button">
          {scanning ? "扫描中..." : "扫描开发工具目录"}
        </button>
      </section>

      <section className="panel wide">
        <h2>环境变量迁移目标</h2>
        <div className="field-row">
          <input
            aria-label="开发工具环境变量目标根目录"
            onChange={(event) => setTargetRoot(event.currentTarget.value)}
            value={targetRoot}
          />
        </div>
        <p>环境变量迁移会创建目标目录并修改用户级环境变量；Junction 预案只生成目标路径和风险检查，不移动文件。</p>
      </section>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>规则项</span>
          <strong>{results.length || rules.length}</strong>
        </div>
        <div className="metric">
          <span>命中目录</span>
          <strong>{existingResults.length}</strong>
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

      <section className="panel">
        <h2>规则文件工具项</h2>
        {error ? <p className="error-text">{error}</p> : null}
        <ul className="placeholder-list">
          {rules.map((rule) => (
            <li key={rule.id}>
              <span>
                {rule.name}
                <small>{rule.paths.join(", ")}</small>
                {Object.keys(rule.env ?? {}).length ? (
                  <small>环境变量：{Object.keys(rule.env).join(", ")}</small>
                ) : null}
              </span>
              <span className="tag">{rule.preferredMove}</span>
              <button
                className="inline-action"
                disabled={rule.preferredMove !== "env" || planningId === rule.id}
                onClick={() => createEnvPlan(rule.id)}
                title={rule.preferredMove === "env" ? "生成环境变量迁移计划" : "该工具不支持环境变量迁移"}
                type="button"
              >
                {planningId === rule.id ? "计划中" : "环境变量计划"}
              </button>
              <button
                className="inline-action"
                disabled={
                  !(rule.preferredMove === "junction" || rule.fallback === "junction") ||
                  rule.risk === "high" ||
                  planningId === rule.id
                }
                onClick={() => createJunctionPlan(rule.id)}
                title={
                  rule.risk === "high"
                    ? "高风险目录只读展示"
                    : rule.preferredMove === "junction" || rule.fallback === "junction"
                      ? "生成 Junction 迁移预案"
                      : "该工具不支持 Junction 预案"
                }
                type="button"
              >
                {planningId === rule.id ? "计划中" : "Junction 预案"}
              </button>
              <button
                className="danger-action compact"
                disabled={
                  !plans[rule.id]?.canExecute ||
                  confirmations[rule.id] !== "MIGRATE_DEVSPACE_ENV" ||
                  executingId === rule.id
                }
                onClick={() => executeEnvMigration(rule.id)}
                title={
                  !plans[rule.id]
                    ? "需要先生成环境变量迁移计划"
                    : confirmations[rule.id] !== "MIGRATE_DEVSPACE_ENV"
                      ? "需要输入确认短语"
                      : "修改用户级环境变量"
                }
                type="button"
              >
                {executingId === rule.id ? "迁移中" : "执行"}
              </button>
              <button
                className="danger-action compact"
                disabled={
                  !junctionPlans[rule.id]?.canExecute ||
                  confirmations[`junction-${rule.id}`] !== "CREATE_DEVSPACE_JUNCTION" ||
                  executingId === rule.id
                }
                onClick={() => executeJunctionMigration(rule.id)}
                title={
                  !junctionPlans[rule.id]
                    ? "需要先生成 Junction 预案"
                    : confirmations[`junction-${rule.id}`] !== "CREATE_DEVSPACE_JUNCTION"
                      ? "需要输入 Junction 确认短语"
                      : "执行 Junction 迁移"
                }
                type="button"
              >
                {executingId === rule.id ? "执行中" : "执行 Junction"}
              </button>
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>敏感目录</h2>
        <p>含账号、密钥、授权信息的目录默认只扫描和提示，不自动迁移。</p>
      </section>

      <section className="panel wide">
        <h2>Junction 迁移预案</h2>
        <ul className="placeholder-list">
          {Object.values(junctionPlans).map((plan) => (
            <li key={plan.id}>
              <span>
                {plan.name}
                <small>目标根目录：{plan.targetRoot}</small>
                {plan.blockers.length ? <small>阻塞：{plan.blockers.join("，")}</small> : null}
                {plan.warnings.length ? <small>警告：{plan.warnings.join("，")}</small> : null}
                {plan.items.map((item) => (
                  <small key={item.sourcePath}>
                    {item.sourcePath} {"->"} {item.targetPath} / {formatBytes(item.sourceBytes)} /{" "}
                    {item.alreadyJunction ? "已是 Junction" : "普通路径"}
                    {item.blockers.length ? ` / 阻塞：${item.blockers.join("，")}` : ""}
                    {item.warnings.length ? ` / 警告：${item.warnings.join("，")}` : ""}
                  </small>
                ))}
                {plan.canExecute ? (
                  <small>
                    <input
                      aria-label={`${plan.name} Junction 迁移确认短语`}
                      onChange={(event) => {
                        const value = event.currentTarget.value;
                        setConfirmations((current) => ({
                          ...current,
                          [`junction-${plan.targetId}`]: value,
                        }));
                      }}
                      placeholder="输入 CREATE_DEVSPACE_JUNCTION 后可执行"
                      value={confirmations[`junction-${plan.targetId}`] ?? ""}
                    />
                  </small>
                ) : null}
                {junctionResults[plan.targetId] ? (
                  <small>
                    已迁移 {junctionResults[plan.targetId].movedCount} 项，跳过{" "}
                    {junctionResults[plan.targetId].skippedCount} 项
                  </small>
                ) : null}
                {junctionResults[plan.targetId]?.failures.length ? (
                  <small>
                    失败项：
                    {junctionResults[plan.targetId].failures
                      .map((failure) => `${failure.path}: ${failure.reason}`)
                      .join("；")}
                  </small>
                ) : null}
              </span>
              <span className="tag">
                {junctionResults[plan.targetId]?.status ?? (plan.canExecute ? "ready" : "blocked")}
              </span>
            </li>
          ))}
        </ul>
      </section>

      <section className="panel wide">
        <h2>扫描结果</h2>
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
                <small>
                  {result.category} / {result.preferredAction} / {result.preferredMove}
                </small>
              </span>
              <span className="tag">
                {result.exists ? `${formatBytes(result.totalBytes)} / ${result.risk}` : "missing"}
              </span>
            </li>
          ))}
        </ul>
      </section>

      <section className="panel wide">
        <h2>环境变量迁移计划与结果</h2>
        <ul className="placeholder-list">
          {Object.values(plans).map((plan) => {
            const result = migrationResults[plan.targetId];
            return (
              <li key={plan.id}>
                <span>
                  {plan.name}
                  {plan.variables.map((variable) => (
                    <small key={variable.name}>
                      {variable.name}: {variable.originalValue ?? "(未设置)"} {"->"} {variable.newValue}
                    </small>
                  ))}
                  {plan.blockers.length ? <small>阻塞：{plan.blockers.join("，")}</small> : null}
                  {plan.warnings.length ? <small>警告：{plan.warnings.join("，")}</small> : null}
                  {plan.canExecute ? (
                    <small>
                      <input
                        aria-label={`${plan.name} 环境变量迁移确认短语`}
                        onChange={(event) => {
                          const value = event.currentTarget.value;
                          setConfirmations((current) => ({
                            ...current,
                            [plan.targetId]: value,
                          }));
                        }}
                        placeholder="输入 MIGRATE_DEVSPACE_ENV 后可执行"
                        value={confirmations[plan.targetId] ?? ""}
                      />
                    </small>
                  ) : null}
                  {result ? (
                    <small>
                      已修改 {result.changedCount} 项，跳过 {result.skippedCount} 项
                    </small>
                  ) : null}
                  {result?.failures.length ? (
                    <small>
                      失败项：{result.failures.map((failure) => `${failure.path}: ${failure.reason}`).join("；")}
                    </small>
                  ) : null}
                </span>
                <span className="tag">{result?.status ?? (plan.canExecute ? "ready" : "blocked")}</span>
              </li>
            );
          })}
        </ul>
      </section>
    </div>
  );
}
