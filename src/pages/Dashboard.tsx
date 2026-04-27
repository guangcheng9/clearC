import { useEffect, useState } from "react";
import { DiskOverview, formatBytes, getDiskOverview } from "../lib/scan";

type DashboardProps = {
  status:
    | {
        appName: string;
        version: string;
        nativeCoreReady: boolean;
        rulesPath: string;
      }
    | null;
};

export function Dashboard({ status }: DashboardProps) {
  const [disk, setDisk] = useState<DiskOverview | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    getDiskOverview()
      .then(setDisk)
      .catch((err) => setError(String(err)));
  }, []);

  const usedPercent = disk?.totalBytes ? Math.round((disk.usedBytes / disk.totalBytes) * 100) : 0;

  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>系统盘概览</h2>
        <p>
          当前阶段接入只读扫描能力，只读取容量和规则路径占用，不执行清理、删除或迁移操作。
        </p>
        {error ? <p className="error-text">{error}</p> : null}
      </section>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>系统盘</span>
          <strong>{disk?.drive ?? "读取中"}</strong>
        </div>
        <div className="metric">
          <span>总容量</span>
          <strong>{formatBytes(disk?.totalBytes ?? 0)}</strong>
        </div>
        <div className="metric">
          <span>已用</span>
          <strong>{formatBytes(disk?.usedBytes ?? 0)}</strong>
        </div>
        <div className="metric">
          <span>可用</span>
          <strong>{formatBytes(disk?.freeBytes ?? 0)}</strong>
        </div>
      </div>

      <section className="panel wide">
        <h2>使用率</h2>
        <div className="progress-track" aria-label="系统盘使用率">
          <div className="progress-fill" style={{ width: `${usedPercent}%` }} />
        </div>
        <p>{usedPercent}% 已使用</p>
      </section>

      <section className="panel">
        <h2>核心状态</h2>
        <ul className="placeholder-list">
          <li>
            {status?.appName ?? "ClearC"} <span className="tag">{status?.version ?? "0.1.0"}</span>
          </li>
          <li>
            Native Core <span className="tag">{status?.nativeCoreReady ? "ready" : "pending"}</span>
          </li>
          <li>
            规则目录 <span className="tag">{status?.rulesPath ?? "rules"}</span>
          </li>
        </ul>
      </section>

      <section className="panel">
        <h2>当前步骤</h2>
        <p>`V1 / M2` 正在实现扫描系统：磁盘概览、规则路径扫描、大小统计和失败项记录。</p>
      </section>
    </div>
  );
}
