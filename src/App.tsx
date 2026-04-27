import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { ErrorBoundary } from "./ErrorBoundary";
import { Analysis } from "./pages/Analysis";
import { Cleanup } from "./pages/Cleanup";
import { Dashboard } from "./pages/Dashboard";
import { DevSpace } from "./pages/DevSpace";
import { Logs } from "./pages/Logs";
import { Migration } from "./pages/Migration";

type PageKey = "dashboard" | "cleanup" | "migration" | "devspace" | "analysis" | "logs";

type AppStatus = {
  appName: string;
  version: string;
  nativeCoreReady: boolean;
  rulesPath: string;
};

const pages: Array<{ key: PageKey; label: string; description: string }> = [
  { key: "dashboard", label: "首页", description: "系统盘概览与建议" },
  { key: "cleanup", label: "清理", description: "缓存、临时文件、回收站" },
  { key: "migration", label: "迁移", description: "用户默认目录迁移" },
  { key: "devspace", label: "开发工具", description: "AI 工具、SDK、包管理器" },
  { key: "analysis", label: "分析", description: "大文件与目录占用" },
  { key: "logs", label: "日志", description: "操作记录与回滚" },
];

function App() {
  const [activePage, setActivePage] = useState<PageKey>("dashboard");
  const [status, setStatus] = useState<AppStatus | null>(null);

  useEffect(() => {
    invoke<AppStatus>("get_app_status")
      .then(setStatus)
      .catch(() => {
        setStatus({
          appName: "ClearC",
          version: "0.1.0",
          nativeCoreReady: false,
          rulesPath: "rules",
        });
      });
  }, []);

  const page = useMemo(() => {
    switch (activePage) {
      case "cleanup":
        return <Cleanup />;
      case "migration":
        return <Migration />;
      case "devspace":
        return <DevSpace />;
      case "analysis":
        return <Analysis />;
      case "logs":
        return <Logs />;
      default:
        return <Dashboard status={status} />;
    }
  }, [activePage, status]);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">C</div>
          <div>
            <strong>ClearC</strong>
            <span>系统盘治理工具</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="主导航">
          {pages.map((item) => (
            <button
              className={item.key === activePage ? "nav-item active" : "nav-item"}
              key={item.key}
              onClick={() => setActivePage(item.key)}
              type="button"
            >
              <span>{item.label}</span>
              <small>{item.description}</small>
            </button>
          ))}
        </nav>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h1>{pages.find((item) => item.key === activePage)?.label}</h1>
            <p>{pages.find((item) => item.key === activePage)?.description}</p>
          </div>
          <div className={status?.nativeCoreReady ? "status ready" : "status"}>
            Native Core {status?.nativeCoreReady ? "Ready" : "Pending"}
          </div>
        </header>
        <ErrorBoundary>{page}</ErrorBoundary>
      </section>
    </main>
  );
}

export default App;
