# riff-ui

用 Rust 在 gpui-kit 上复现团队的 React 组件库 riff-uiux。只放组件库，不放应用代码。背景、结构和运行方式见 [`README.md`](README.md)，复现进度见 [`docs/components.md`](docs/components.md)，接下来做什么见 [`docs/roadmap.md`](docs/roadmap.md)（做完一个组件就更新它）。

## 原库

- 位置：`/home/yuki/Downloads/riff-uiux`，团队内部代码，只读、只作参考：不复制进本仓库，不提交、不推到任何远程，不把大段源码贴到外部服务。
- 可以当规格的：`docs/<slug>.md`、`src/components/<slug>.tsx` 里的 CSS、`src/styles/tokens.css`、`docs/tokens.report.md`。
- 不能当规格的：`src/internal/core.tsx`，以及还残留 `X$1` 这类压缩名的屏幕级组件。
- 一切以原库的 API、行为和视觉为准，不照搬 shadcn，也不自己发挥。GPUI 做不到、需要取舍的视觉问题，先列方案问用户，不擅自降级。

## 新增一个组件

一次只做一个组件。每个组件都要能在 story 的 Compare 页上和原库逐格对照。

1. **组件**：`crates/riff-ui/src/<slug>/`。默认用 gpui-base 的行为、自己画样式（见 `button` 模块的写法）；只有原库和 shadcn 风格接近时才包装 gpui-component，并在 `docs/components.md` 写明原因。
2. **用例**：`stories/<slug>.json`，原库和 GPUI 两边共用。按 分区 → 行 → 格 组织，每格是纯数据 props；行可以写 `"cells": "@<模板>"` 并给出整行共享的 props；需要固定悬停、按下、焦点的格子写 `"state": "hover" | "active" | "focus"`；有循环动画的写 `freeze_ms`。覆盖原库文档里的每个 variant、尺寸和状态。
3. **原库适配器**：`tools/reference/adapters/<slug>.jsx`，把 props 翻译成原库组件（`@riff/component`），图标用 `@riff/gallery` 的 `Icon`。有循环动画时导出 `freezeCss`，把动画停在 `@@FREEZE_MS@@`。
4. **GPUI 适配**：`crates/riff-story/src/components/<slug>.rs`，提供 `cell(id, props, pin)`：把 props 翻译成组件，遇到不认识的 prop 或取值返回 `Err`，并按 `pin` 应用固定状态和冻结时刻。可选提供 `DEMO`（自由交互演示）。在 `components/mod.rs` 里声明模块。
5. **登记**：`crates/riff-story/src/registry.rs` 的 `entries()` 里加一行 `entry!("<slug>", <模块>)`。
6. **生成截图并对照**：`python3 tools/reference/build.py <slug>`，然后 `cargo run -- <slug>`，亮暗两种都看。
7. 在 `docs/components.md` 里登记一行。

`cargo test --workspace` 里的 `every_story_cell_builds` 会逐格构建所有用例：用例写了 GPUI 那边不认识的 prop 或取值，测试就失败。

## 交付

- 交付前跑：`cargo test --workspace`、`cargo clippy --workspace --all-targets`、`cargo fmt --all -- --check`、`python3 tools/reference/build.py --check`。
- 交付亮色、暗色两张截图，说明对照了哪些规格、哪些地方没能还原、原因是什么；真实交互说明怎么验证的。
- 只有用户明确说了，才提交和推送。
- 直接提交到 `main` 并推送，不开 PR；本仓库没有 CI/CD。

## 环境与约束

- gpui-kit 锁定在 `=0.10.0`，`Cargo.lock` 里 wgpu 锁定为 30.0.1；不要随手 `cargo update`。新增依赖优先选锁文件里已有的版本。
- 字体用仓库自带的 Inter（`crates/riff-ui/assets/fonts/`，原库截图也用同一份），不依赖系统字体。
- 截图：`tools/shoot.sh <输出.png> [riff-story 参数…]` 启动 story、用 niri 截窗口、关掉自己启动的进程。截图放在 `target/shots/`（git 忽略）。
