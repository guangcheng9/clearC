export function Migration() {
  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>用户目录迁移</h2>
        <p>这里会处理下载、文档、图片、视频、音乐、桌面等 Windows 默认用户目录迁移。</p>
      </section>
      <section className="panel">
        <h2>迁移方式</h2>
        <p>优先读取和修改 Windows Shell Folder 配置，并记录原路径、新路径和回滚信息。</p>
      </section>
      <section className="panel">
        <h2>预检查</h2>
        <p>迁移前检查目标盘空间、路径权限、文件占用、路径冲突和是否为移动设备。</p>
      </section>
    </div>
  );
}
