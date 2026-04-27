import { useEffect, useState } from "react";
import { DevSpaceScanResult, scanDevSpaceTargets } from "../lib/devspace";
import { DevSpaceRule, getRuleCatalog } from "../lib/rules";
import { formatBytes } from "../lib/scan";

export function DevSpace() {
  const [rules, setRules] = useState<DevSpaceRule[]>([]);
  const [results, setResults] = useState<DevSpaceScanResult[]>([]);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    getRuleCatalog()
      .then((catalog) => setRules(catalog.devspace))
      .catch((err) => setError(String(err)));
  }, []);

  function runScan() {
    setScanning(true);
    setError("");
    scanDevSpaceTargets()
      .then(setResults)
      .catch((err) => setError(String(err)))
      .finally(() => setScanning(false));
  }

  const existingResults = results.filter((result) => result.exists);
  const totalBytes = results.reduce((sum, result) => sum + result.totalBytes, 0);
  const skippedCount = results.reduce((sum, result) => sum + result.skippedCount, 0);

  return (
    <div className="page-grid">
      <section className="panel wide" aria-busy={scanning}>
        <h2>开发工具空间管理</h2>
        <p>当前阶段只扫描和统计开发工具、AI 工具、SDK、包管理器缓存，不清理、不迁移。</p>
        {scanning ? <p className="busy-text">正在扫描开发工具目录，路径较大时可能需要一些时间。</p> : null}
        <button className="primary-action" disabled={scanning} onClick={runScan} type="button">
          {scanning ? "扫描中..." : "扫描开发工具目录"}
        </button>
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
              </span>
              <span className="tag">{rule.preferredMove}</span>
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>敏感目录</h2>
        <p>含账号、密钥、授权信息的目录默认只扫描和提示，不自动迁移。</p>
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
    </div>
  );
}
