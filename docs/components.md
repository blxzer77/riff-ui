# 组件对照表

团队原 React 组件库 → riff-ui 的复现进度。

底座取值：
- `base`：基于 gpui-base 的行为，自己写样式和结构（默认）。
- `component`：包装 gpui-component。
- `自研`：两者都不用。

| 原库组件 | riff-ui 模块 | 底座 | 状态 | 备注 |
| -------- | ------------ | ---- | ---- | ---- |
| `styles/tokens.css` | `theme` | 自研 | 完成 | 对齐 2026-11-20 15:11 版的 39 个 token。`--riff-font-mono` / `--riff-mono` 未实现，用 gpui-component 的等宽字体。同时同步到 gpui-component 的 `Theme` |
| `button` | `button` | base | 完成 | 行为用 gpui-base `Button`，外观全部自绘（按下缩到 97% 需要整体缩放，GPUI 的 div 做不到）。已与原库逐格对齐（见下）。模糊淡入用多层高斯采样近似；截断后悬停显示全文未实现（等 tooltip） |
| `icon-button` | `icon_button` | base | 完成 | 普通按钮用 gpui-base `Button`，设 `pressed` 时切为 gpui-base `Toggle`（提供 `aria-pressed`）；外观自绘（含 `:active` 95% 缩放、`reduce_motion` 下跳过缩放）。已与原库 35 格（亮/暗 Stacked 与 Overlay）逐格对齐。tooltip 暂借 gpui-component `Tooltip`（待阶段 2 `tooltip` 完成后替换） |
| `badge` | `badge` | 自研 | 完成 | 六色调胶囊标签（`neutral` / `info` / `success` / `warning` / `danger` / `purple`），支持 `sm`/`md`、`dot`、`live`（1.8s 脉冲光环，支持冻结帧与 `reduce_motion`）、`icon`、等宽数字 `count`（`max+` 上限与千分位分组）及空值不渲染。已与原库 47 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `kbd` | `kbd` | 自研 | 完成 | 快捷键按键帽（单键与组合键），支持 `sm`/`md`、`mac`/`other` 平台映射（`mod` 在 Mac 为 `⌘`、其余为 `Ctrl`）及 1px 立体底边（`inset -1px`），屏幕阅读器自动拼读完整键名。已与原库 56 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `separator` | `separator` | 自研 | 完成 | 水平/垂直 1px 分隔线，支持居中标签（`12.5px`/`18px` 行高、`12px` 间距与最小线宽、80% 最大宽度折行）、`vertical` 默认 18px 居中及自定义长度、`decorative` 控制 `role="separator"` 语义及方向/名称。已与原库 14 格（亮/暗 Stacked 与 Overlay）对照对齐 |

### button 与原库的对齐

逐格对比 Compare 页的 60 格（亮暗各一套，1× 截图逐像素相减）：

- 尺寸：每格与原库相差 0.4px 以内（GPUI 对齐到整像素）。
- 填充色、渐变、hover / pressed / disabled / loading 着色：与原库一致。
- 边缘：secondary、ghost、destructive 一致；primary 静止状态平均差异 1–2%。
- 文字：字体、字号、字重、字距、基线位置一致。基线按浏览器规则取整（行框上下沿各自取整；落在 .5 像素时向下对齐，GPUI 默认向上）。

为对齐而做的处理：

- `letter-spacing: -0.01em` 在逐字绘制时实现（GPUI 没有这个属性）。
- 外扩阴影的圆角按 CSS 规则取「圆角 + 外扩」（GPUI 沿用元素自己的圆角）。
- 半透明（disabled）时边框带和填充分开画、互不重叠（GPUI 按图元分别乘透明度，CSS 是整体合成后再乘）。
- 按下缩放时改用抗锯齿路径绘制，边缘能落在小数像素上（GPUI 的矩形和阴影会对齐到整像素）。

有意保留的差异：

- **文字像素**：GPUI 在本机用彩色次像素抗锯齿，Chrome 截图用灰度抗锯齿，小字号的字形微调也不同。字形位置和宽度一致，像素不同。
- **primary 圆角的浅色缝**：原库在深色外圈和按钮之间沿圆角有一条约 1px 的浅缝，是浏览器把两条弧线分别抗锯齿造成的渲染瑕疵，不是设计的一部分，不复刻。
- **按下时的边缘一行像素**：GPUI 路径用 4× MSAA，只有 5 级覆盖率，浏览器是精确覆盖率；外圈和按钮共用的那一行像素的明暗会略有不同。
