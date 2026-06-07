interface SkillInfo {
  id: string
  name: string
  desc: string
  trigger: string
}

const SKILLS: SkillInfo[] = [
  { id: 'code-review-cn', name: '中文代码审查', desc: '安全、性能、可维护性分级评审', trigger: '审查代码变更' },
  { id: 'refactor-advisor', name: '重构建议', desc: '代码异味识别和重构方案', trigger: '分析现有代码' },
  { id: 'test-generator', name: '测试生成', desc: '单元/集成测试用例生成', trigger: '生成测试用例' },
  { id: 'pr-generator', name: 'PR 生成', desc: '中文 PR 标题和描述生成', trigger: '准备 PR' },
  { id: 'bug-diagnose', name: 'Bug 诊断', desc: '错误日志根因分析', trigger: '贴入错误信息' },
  { id: 'api-scaffold', name: 'API 骨架', desc: 'RESTful API 代码生成', trigger: '创建 API 端点' },
  { id: 'doc-generator', name: '文档生成', desc: '中文技术文档注释', trigger: '生成文档' },
  { id: 'git-workflow', name: 'Git 工作流', desc: '分支策略和冲突解决', trigger: 'Git 操作指导' },
]

function esc(value: unknown): string {
  return String(value ?? '').replace(/[&<>"]/g, ch =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' } as Record<string, string>)[ch] ?? ch
  )
}

export async function render(): Promise<HTMLElement> {
  const page = document.createElement('div')
  page.className = 'page codewhale-skills'

  const cards = SKILLS.map(s => `
    <section class="cw-card cw-skill-card">
      <div class="cw-card-title">${esc(s.name)}</div>
      <div class="cw-skill-id"><code>${esc(s.id)}</code></div>
      <p>${esc(s.desc)}</p>
      <div class="cw-skill-trigger"><small>触发：${esc(s.trigger)}</small></div>
    </section>
  `).join('')

  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>技能管理</h1>
        <p class="page-desc">
          CodeWhale 技能存放在 <code>CODEWHALE_HOME/skills/</code>，自动发现。
          兼容 <code>~/.agents/skills/</code> 和 <code>~/.claude/skills/</code>。
        </p>
      </div>
    </div>
    <div class="cw-grid">${cards}</div>
  `

  return page
}
