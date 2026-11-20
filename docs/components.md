# 组件对照表

团队原 React 组件库 → riff-ui 的复现进度。

底座取值：
- `base`：基于 gpui-base 的行为，自己写样式和结构（默认）。
- `component`：包装 gpui-component。
- `自研`：两者都不用。

| 原库组件 | riff-ui 模块 | 底座 | 状态 | 备注 |
| -------- | ------------ | ---- | ---- | ---- |
| `styles/tokens.css` | `theme` | 自研 | 完成 | 对齐 2026-11-20 15:11 版的 39 个 token。`--riff-font-mono` / `--riff-mono` 未实现，用 gpui-component 的等宽字体。同时同步到 gpui-component 的 `Theme` |
