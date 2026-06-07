/** CodeWhale 引擎类型定义 */

export interface CodeWhaleStatus {
  root: string
  codewhaleHome: string
  configPath: string
  configExists: boolean
  skillsPath: string
  skillCount: number
  cliPath: string
  cliExists: boolean
  tuiPath: string
  tuiExists: boolean
  envKey: string | null
  envPresent: boolean
  version: string | null
  ready: boolean
}

export interface CodeWhaleRunResult {
  success: boolean
  exitCode: number | null
  stdout: string
  stderr: string
}

export interface PanelConfig {
  codewhale?: {
    sourceDir?: string
    cliPath?: string
    provider?: string
    model?: string
    baseUrl?: string
    providers?: Record<string, ProviderConfig>
  }
  codex?: Record<string, unknown>
  openclaw?: Record<string, unknown>
}

export interface ProviderConfig {
  name: string
  baseUrl: string
  model: string
  envKey: string
}

export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  model?: string
  timestamp: number
}

export interface EngineNavItem {
  route: string
  label: string
  icon: string
  gate?: string
}

export interface EngineNavSection {
  section: string
  items: EngineNavItem[]
}

export interface EngineRoute {
  path: string
  loader: () => Promise<{ render: () => Promise<HTMLElement>; cleanup?: () => void }>
}

export interface EnginePlugin {
  id: string
  name: string
  description: string
  icon: string
  detect(): Promise<{ installed: boolean; ready: boolean }>
  boot(): Promise<void>
  cleanup(): void
  getNavItems(): EngineNavSection[]
  getRoutes(): EngineRoute[]
  getSetupRoute(): string
  getDefaultRoute(): string
  isReady(): boolean
  isGatewayRunning(): boolean
  isGatewayForeign(): boolean
  onStateChange(fn: () => void): () => void
  onReadyChange(fn: () => void): () => void
  isFeatureAvailable(): boolean
}
