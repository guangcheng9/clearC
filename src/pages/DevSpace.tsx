export function DevSpace() {
  return (
    <div className="page-grid">
      <section className="panel wide">
        <h2>开发工具空间管理</h2>
        <p>这里会识别 AI 工具、SDK、包管理器和自动化浏览器缓存，按风险等级给出清理或迁移策略。</p>
      </section>
      <section className="panel">
        <h2>优先支持</h2>
        <ul className="placeholder-list">
          <li>
            npm / pnpm / bun <span className="tag">配置迁移</span>
          </li>
          <li>
            cargo / gradle / android <span className="tag">环境变量</span>
          </li>
          <li>
            AI 工具缓存 <span className="tag">谨慎迁移</span>
          </li>
        </ul>
      </section>
      <section className="panel">
        <h2>敏感目录</h2>
        <p>含账号、密钥、授权信息的目录默认只扫描和提示，不自动迁移。</p>
      </section>
    </div>
  );
}
