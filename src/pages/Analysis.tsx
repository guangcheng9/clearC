import { useEffect, useState } from "react";
import { CleanupScanResult, formatBytes, scanCleanupRules } from "../lib/scan";

export function Analysis() {
  const [results, setResults] = useState<CleanupScanResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    scanCleanupRules()
      .then(setResults)
      .catch((err) => setError(String(err)))
      .finally(() => setLoading(false));
  }, []);

  const totalBytes = results.reduce((sum, item) => sum + item.totalBytes, 0);
  const fileCount = results.reduce((sum, item) => sum + item.fileCount, 0);
  const skippedCount = results.reduce((sum, item) => sum + item.skippedCount, 0);

  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>空间分析</h2>
        <p>当前展示清理规则命中的只读扫描结果。后续会扩展到大文件、大目录和重复文件检测。</p>
        {loading ? <p>扫描中...</p> : null}
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
