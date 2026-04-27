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
  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>项目初始化状态</h2>
        <p>
          当前版本先完成桌面壳、页面框架、规则目录和 Native Core 命令骨架。真实扫描和清理会在后续里程碑逐步接入。
        </p>
      </section>

      <div className="metric-row panel wide">
        <div className="metric">
          <span>应用</span>
          <strong>{status?.appName ?? "ClearC"}</strong>
        </div>
        <div className="metric">
          <span>版本</span>
          <strong>{status?.version ?? "0.1.0"}</strong>
        </div>
        <div className="metric">
          <span>核心层</span>
          <strong>{status?.nativeCoreReady ? "就绪" : "等待"}</strong>
        </div>
        <div className="metric">
          <span>规则目录</span>
          <strong>{status?.rulesPath ?? "rules"}</strong>
        </div>
      </div>

      <section className="panel">
        <h2>V0 目标</h2>
        <ul className="placeholder-list">
          <li>
            页面框架 <span className="tag">已建立</span>
          </li>
          <li>
            规则文件 <span className="tag">已预留</span>
          </li>
          <li>
            Rust 命令 <span className="tag">已预留</span>
          </li>
        </ul>
      </section>

      <section className="panel">
        <h2>下一步</h2>
        <p>接入只读扫描：磁盘容量、规则命中、目录大小统计和扫描进度事件。</p>
      </section>
    </div>
  );
}
