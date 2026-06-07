// @ts-nocheck
export async function render() {
  const skills = [
    { id: 'code-review-cn', name: '中文代码审查', desc: '关注安全性、性能和可维护性，分级评审', trigger: '用户请求审查代码' },
    { id: 'refactor-advisor', name: '重构建议', desc: '识别代码异味，建议重构方案', trigger: '用户请求重构' },
    { id: 'test-generator', name: '测试生成', desc: '生成单元/集成测试用例', trigger: '用户请求生成测试' },
    { id: 'pr-generator', name: 'PR 生成', desc: '生成中文 PR 标题和描述', trigger: '用户请求 PR 描述' },
    { id: 'bug-diagnose', name: 'Bug 诊断', desc: '从错误日志定位根因', trigger: '用户贴入错误信息' },
    { id: 'api-scaffold', name: 'API 骨架', desc: '生成 RESTful API 骨架代码', trigger: '用户请求创建 API' },
    { id: 'doc-generator', name: '文档生成', desc: '生成中文技术文档注释', trigger: '用户请求生成文档' },
    { id: 'git-workflow', name: 'Git 工作流', desc: '分支策略和冲突解决指导', trigger: '用户请求 Git 操作' },
  ]

  const page = document.createElement('div')
  page.className = 'page codex-skills'
  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>技能管理</h1>
        <p class="page-desc">Codex 内置编码技能，存放在 <code>CODEX_HOME/skills/</code>，自动发现无需手动注册。</p>
      </div>
    </div>
    <div class="codex-grid">
      ${skills.map(s => `
        <section class="codex-panel codex-skill-card">
          <div class="codex-panel-title">${s.name}</div>
          <div class="codex-skill-meta"><code>${s.id}</code></div>
          <p>${s.desc}</p>
          <div class="codex-skill-trigger"><small>触发：${s.trigger}</small></div>
        </section>
      `).join('')}
    </div>
  `

  return page
}
