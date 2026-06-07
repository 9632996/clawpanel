import test from 'node:test'
import assert from 'node:assert/strict'

class FakeHTMLElement {
  constructor({ id = '', action = '', label = '', disabled = false } = {}) {
    this.id = id
    this.disabled = disabled
    this.dataset = {}
    this.attrs = new Map()
    this.classNames = new Set()
    if (action) this.dataset.action = action
    if (label) this.dataset.label = label
    this.classList = {
      add: (...names) => names.forEach(name => this.classNames.add(name)),
      remove: (...names) => names.forEach(name => this.classNames.delete(name)),
      contains: name => this.classNames.has(name),
    }
  }

  getAttribute(name) {
    return this.attrs.get(name) ?? null
  }

  setAttribute(name, value) {
    this.attrs.set(name, String(value))
  }

  removeAttribute(name) {
    this.attrs.delete(name)
  }
}

globalThis.window = { location: { hostname: 'localhost' } }
globalThis.HTMLElement = FakeHTMLElement

const appState = await import('../src/lib/app-state.ts')

function createRoot() {
  const buttons = {
    dashboardStop: new FakeHTMLElement({ action: 'stop-gw' }),
    dashboardRestart: new FakeHTMLElement({ action: 'restart-gw' }),
    serviceStop: new FakeHTMLElement({ action: 'stop', label: 'ai.openclaw.gateway' }),
    serviceRestart: new FakeHTMLElement({ action: 'restart', label: 'ai.openclaw.gateway' }),
    start: new FakeHTMLElement({ action: 'start-gw' }),
  }
  return {
    buttons,
    root: {
      querySelectorAll: () => Object.values(buttons),
    },
  }
}

function isLoading(button) {
  return button.classList.contains('btn-loading')
}

async function withPendingGatewayOperation(action, fn) {
  let release
  const pending = appState.runGatewayOperation(
    action,
    () => new Promise(resolve => { release = resolve }),
    { label: `${action} in progress` },
  )
  try {
    await fn()
  } finally {
    release()
    await pending
  }
}

test('stop gateway operation only shows loading on stop buttons', async () => {
  const { root, buttons } = createRoot()

  await withPendingGatewayOperation('stop', async () => {
    appState.syncGatewayActionButtons(root)

    assert.equal(isLoading(buttons.dashboardStop), true)
    assert.equal(isLoading(buttons.serviceStop), true)
    assert.equal(isLoading(buttons.dashboardRestart), false)
    assert.equal(isLoading(buttons.serviceRestart), false)
    assert.equal(isLoading(buttons.start), false)
    assert.equal(buttons.dashboardRestart.disabled, true)
  })
})

test('restart gateway operation only shows loading on restart buttons', async () => {
  const { root, buttons } = createRoot()

  await withPendingGatewayOperation('restart', async () => {
    appState.syncGatewayActionButtons(root)

    assert.equal(isLoading(buttons.dashboardRestart), true)
    assert.equal(isLoading(buttons.serviceRestart), true)
    assert.equal(isLoading(buttons.dashboardStop), false)
    assert.equal(isLoading(buttons.serviceStop), false)
    assert.equal(isLoading(buttons.start), false)
    assert.equal(buttons.dashboardStop.disabled, true)
  })
})

