import { useEffect, useState } from "react";
import {
  createUserFolderMigrationPlan,
  executeUserFolderMigration,
  getUserFolderTargets,
  MigrationPrecheck,
  precheckUserFolderMigration,
  UserFolderMigrationPlan,
  UserFolderMigrationResult,
  UserFolderTarget,
} from "../lib/migration";
import { getRuleCatalog, MigrationRule } from "../lib/rules";
import { formatBytes } from "../lib/scan";

export function Migration() {
  const [rules, setRules] = useState<MigrationRule[]>([]);
  const [targets, setTargets] = useState<UserFolderTarget[]>([]);
  const [targetRoot, setTargetRoot] = useState("D:\\ClearC-UserFolders");
  const [prechecks, setPrechecks] = useState<Record<string, MigrationPrecheck>>({});
  const [plans, setPlans] = useState<Record<string, UserFolderMigrationPlan>>({});
  const [migrationResults, setMigrationResults] = useState<Record<string, UserFolderMigrationResult>>({});
  const [confirmations, setConfirmations] = useState<Record<string, string>>({});
  const [checkingId, setCheckingId] = useState("");
  const [planningId, setPlanningId] = useState("");
  const [executingId, setExecutingId] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    getRuleCatalog()
      .then((catalog) => setRules(catalog.migration))
      .catch((err) => setError(String(err)));

    getUserFolderTargets()
      .then(setTargets)
      .catch((err) => setError(String(err)));
  }, []);

  function runPrecheck(folderId: string) {
    setCheckingId(folderId);
    setError("");
    precheckUserFolderMigration(folderId, targetRoot)
      .then((result) => {
        setPrechecks((current) => ({
          ...current,
          [folderId]: result,
        }));
      })
      .catch((err) => setError(String(err)))
      .finally(() => setCheckingId(""));
  }

  function createPlan(folderId: string) {
    setPlanningId(folderId);
    setError("");
    createUserFolderMigrationPlan(folderId, targetRoot)
      .then((result) => {
        setPlans((current) => ({
          ...current,
          [folderId]: result,
        }));
      })
      .catch((err) => setError(String(err)))
      .finally(() => setPlanningId(""));
  }

  function executeMigration(folderId: string) {
    const plan = plans[folderId];
    const confirmation = confirmations[folderId] ?? "";
    if (!plan?.canExecute || confirmation !== "MIGRATE_USER_FOLDER") {
      setError("执行迁移前需要先生成可执行计划，并输入确认短语 MIGRATE_USER_FOLDER。");
      return;
    }

    const accepted = window.confirm(
      `确认迁移 ${plan.name}？\n\n源路径：${plan.sourcePath}\n目标路径：${plan.targetPath}\n\n迁移会移动源目录直接子项并修改 Windows User Shell Folders。`
    );
    if (!accepted) {
      return;
    }

    setExecutingId(folderId);
    setError("");
    executeUserFolderMigration(folderId, targetRoot, confirmation)
      .then((result) => {
        setMigrationResults((current) => ({
          ...current,
          [folderId]: result,
        }));
        getUserFolderTargets().then(setTargets).catch((err) => setError(String(err)));
      })
      .catch((err) => setError(String(err)))
      .finally(() => setExecutingId(""));
  }

  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>用户目录迁移</h2>
        <p>当前阶段支持在用户确认后迁移 Windows 默认用户目录。执行前必须通过预检查、生成计划并输入确认短语。</p>
      </section>
      <section className="panel wide">
        <h2>目标根目录</h2>
        <div className="field-row">
          <input
            aria-label="目标根目录"
            onChange={(event) => setTargetRoot(event.currentTarget.value)}
            value={targetRoot}
          />
        </div>
        <p>预检查只读取路径和容量状态，不创建目录。</p>
        {checkingId ? <p className="busy-text">正在预检查目标路径，请稍候。</p> : null}
        {planningId ? <p className="busy-text">正在生成迁移计划。</p> : null}
        {executingId ? <p className="busy-text">正在执行迁移，请不要重复点击。</p> : null}
      </section>
      <section className="panel wide">
        <h2>当前用户目录</h2>
        {error ? <p className="error-text">{error}</p> : null}
        <ul className="placeholder-list">
          {targets.map((target) => (
            <li key={target.id}>
              <span>
                {target.name}
                <small>{target.resolvedPath}</small>
              </span>
              <span className="tag">
                {target.exists ? (target.onSystemDrive ? "C盘" : "已迁出") : "缺失"}
              </span>
              <button
                className="inline-action"
                disabled={checkingId === target.id}
                onClick={() => runPrecheck(target.id)}
                type="button"
              >
                {checkingId === target.id ? "检查中" : "预检查"}
              </button>
              <button
                className="inline-action"
                disabled={!prechecks[target.id]?.canContinue || planningId === target.id}
                onClick={() => createPlan(target.id)}
                type="button"
              >
                {planningId === target.id ? "计划中" : "生成计划"}
              </button>
              <button
                className="danger-action compact"
                disabled={
                  !plans[target.id]?.canExecute ||
                  confirmations[target.id] !== "MIGRATE_USER_FOLDER" ||
                  executingId === target.id
                }
                onClick={() => executeMigration(target.id)}
                type="button"
                title={
                  !plans[target.id]
                    ? "需要先生成迁移计划"
                    : confirmations[target.id] !== "MIGRATE_USER_FOLDER"
                      ? "需要输入确认短语"
                      : "执行迁移"
                }
              >
                {executingId === target.id ? "迁移中" : "执行迁移"}
              </button>
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>规则文件迁移项</h2>
        {error ? <p className="error-text">{error}</p> : null}
        <ul className="placeholder-list">
          {rules.map((rule) => (
            <li key={rule.id}>
              <span>
                {rule.name}
                <small>{rule.source}</small>
              </span>
              <span className="tag">{rule.strategy}</span>
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>预检查</h2>
        <ul className="placeholder-list">
          {targets.map((target) => {
            const precheck = prechecks[target.id];
            return (
              <li key={`${target.id}-check`}>
                <span>
                  {target.name}
                  <small>
                    {precheck
                      ? `${precheck.targetPath} / ${formatBytes(precheck.sourceBytes)}`
                      : `注册表值：${target.registryValue}`}
                  </small>
                  {precheck?.blockers.length ? (
                    <small>阻塞：{precheck.blockers.join("，")}</small>
                  ) : null}
                  {precheck?.warnings.length ? (
                    <small>警告：{precheck.warnings.join("，")}</small>
                  ) : null}
                </span>
                <span className="tag">
                  {precheck ? (precheck.canContinue ? "通过" : "阻塞") : target.status}
                </span>
              </li>
            );
          })}
        </ul>
      </section>

      <section className="panel">
        <h2>空间状态</h2>
        <ul className="placeholder-list">
          {Object.values(prechecks).map((precheck) => (
            <li key={`${precheck.folderId}-space`}>
              <span>
                {precheck.name}
                <small>目标可用：{formatBytes(precheck.targetFreeBytes)}</small>
              </span>
              <span className="tag">{precheck.hasEnoughSpace ? "space-ok" : "no-space"}</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="panel wide">
        <h2>迁移计划与结果</h2>
        <ul className="placeholder-list">
          {Object.values(plans).map((plan) => {
            const result = migrationResults[plan.folderId];
            return (
              <li key={plan.id}>
                <span>
                  {plan.name}
                  <small>
                    {plan.sourcePath} {"->"} {plan.targetPath}
                  </small>
                  {plan.blockers.length ? <small>阻塞：{plan.blockers.join("，")}</small> : null}
                  {result ? (
                    <small>
                      已移动 {result.movedCount} 项，跳过 {result.skippedCount} 项
                    </small>
                  ) : null}
                  {plan.canExecute ? (
                    <small>
                      <input
                        aria-label={`${plan.name} 迁移确认短语`}
                        onChange={(event) => {
                          const value = event.currentTarget.value;
                          setConfirmations((current) => ({
                            ...current,
                            [plan.folderId]: value,
                          }));
                        }}
                        placeholder="输入 MIGRATE_USER_FOLDER 后可执行"
                        value={confirmations[plan.folderId] ?? ""}
                      />
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
