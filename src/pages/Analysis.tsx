export function Analysis() {
  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>空间分析</h2>
        <p>这里会展示大文件、大目录、下载目录分类和后续重复文件检测结果。</p>
      </section>
      <section className="panel">
        <h2>大文件</h2>
        <p>默认只展示，不自动删除。后续支持按大小、类型和修改时间筛选。</p>
      </section>
      <section className="panel">
        <h2>目录排行</h2>
        <p>帮助用户理解系统盘空间来源，为清理和迁移提供依据。</p>
      </section>
    </div>
  );
}
