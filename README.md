# 智爪平台

`智爪平台` 是 AI作品基于 ClawPanel 二次开发的多智能体产品基座，用于交付面向最终用户的便携式 AI 工作台。

本仓库是 `OpenClawPortable` 项目使用的本地化二开源码，不再作为原 ClawPanel 的普通镜像维护。产品发布时由 builder 读取本仓库源码，打包为便携版运行时。

## 产品定位

- 统一承载 OpenClaw、Hermes Agent、Codex 等多引擎入口。
- 面向中文最终用户提供更低门槛的初始化、模型配置、Agent 预设和技能包能力。
- 配合 AI作品模型服务与工具入口，为客户提供开箱即用的便携版体验。
- 保留 ClawPanel 的可视化管理能力，同时按 AI作品产品规范进行品牌、文案和运行时适配。

## 官方入口

- AI作品主站：<https://aizuopin.com>
- 智爪工具入口：<https://ai.iazp.cn>
- 本项目 fork：<https://github.com/9632996/clawpanel>
- 上游项目：<https://github.com/qingchencloud/clawpanel>

## 与上游的关系

本仓库基于开源项目 ClawPanel 二次开发。上游项目保留其原始版权与开源许可；本仓库中的品牌、本地化、便携运行时、模型服务适配和产品化改造由 AI作品维护。

维护约定：

- `origin` 指向 AI作品二开仓库。
- `upstream` 指向原 ClawPanel 仓库。
- 日常产品迭代直接提交到本仓库。
- 只有在维护周期内才从 `upstream` 合并更新，并逐项核对本地化和产品化改动。
- 不在发布构建中自动覆盖本仓库源码。

## 本地开发

安装依赖：

```powershell
npm install
```

启动前端开发服务：

```powershell
npm run dev
```

构建前端资源：

```powershell
npm run build
```

构建 Tauri 桌面端：

```powershell
npx tauri build --no-bundle --ci
```

## 便携版构建

便携版由上层 `OpenClawPortable` builder 统一编排。本仓库通常位于：

```text
OpenClawPortable/vendor/clawpanel
```

在上层项目中构建到目标目录示例：

```powershell
cargo run -p openclaw-portable-builder -- build-clawpanel-dist --target F:\TestU --force --build-panel --skip-openclaw-build
```

builder 会校验本仓库已经是 `zhizhua-workbench` / `智爪平台` 产品基座，然后叠加少量便携运行时补丁和发布资源。

## 维护规则

- 关于页、标题、托盘、provider、模型服务、官网入口等产品信息，应直接在本仓库源码中维护。
- 不要依赖上层 builder 对 ClawPanel 源码做批量品牌替换。
- 暂不关联原 ClawPanel 社群二维码、Discord、元宝派等入口；后续仅接入 AI作品自己的正式社群入口。
- 更新上游时必须人工比对本地二开改动，完成构建验证后再提交。
- 验收通过后提交 Git；是否推送由发布流程决定。

## 当前内置方向

- OpenClaw：中文增强运行时和便携 Gateway 管理。
- Hermes Agent：多会话、记忆、人格和工具管理入口。
- Codex：作为第三引擎入口预留，后续适配国内模型与便携运行时。
- AI作品模型服务：默认面向 `https://ai.iazp.cn` 的模型服务接入。

## License

本仓库继承上游 ClawPanel 的开源许可要求。使用、分发和二次开发时需同时遵守上游许可证及本仓库中第三方依赖的许可证声明。
