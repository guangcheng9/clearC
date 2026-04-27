export function Logs() {
  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>日志与回滚</h2>
        <p>所有清理和迁移动作都会记录操作时间、路径、大小、成功项、失败项和回滚信息。</p>
      </section>
      <section className="panel">
        <h2>操作日志</h2>
        <p>V1 会先记录清理日志，V2 开始扩展迁移日志和回滚入口。</p>
      </section>
      <section className="panel">
        <h2>回滚策略</h2>
        <p>涉及配置、环境变量、Junction 或 Shell Folder 的修改都必须保存修改前状态。</p>
      </section>
    </div>
  );
}
