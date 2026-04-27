export function Cleanup() {
  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>安全清理</h2>
        <p>这里会承载系统缓存、临时文件、回收站和浏览器缓存的扫描、预览、确认和清理结果。</p>
      </section>
      <section className="panel">
        <h2>低风险清理项</h2>
        <ul className="placeholder-list">
          <li>
            用户临时文件 <span className="tag">V1</span>
          </li>
          <li>
            回收站 <span className="tag">V1</span>
          </li>
          <li>
            缩略图缓存 <span className="tag">V1</span>
          </li>
        </ul>
      </section>
      <section className="panel">
        <h2>执行原则</h2>
        <p>清理前展示明细和预计释放空间，执行时跳过占用和权限不足文件，结束后写入操作日志。</p>
      </section>
    </div>
  );
}
