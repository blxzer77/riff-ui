# 组件对照表

团队原 React 组件库 → riff-ui 的复现进度。

底座取值：
- `base`：基于 gpui-base 的行为，自己写样式和结构（默认）。
- `component`：包装 gpui-component。
- `自研`：两者都不用。

| 原库组件 | riff-ui 模块 | 底座 | 状态 | 备注 |
| -------- | ------------ | ---- | ---- | ---- |
| `styles/tokens.css` | `theme` | 自研 | 完成 | 对齐 2026-11-20 15:11 版的 39 个 token。`--riff-font-mono` / `--riff-mono` 未实现，用 gpui-component 的等宽字体。同时同步到 gpui-component 的 `Theme` |
| `button` | `button` | base | 完成 | 行为用 gpui-base `Button`，外观全部自绘（按下缩到 97% 需要整体缩放，GPUI 的 div 做不到）。已与原库逐格对齐（见下）。模糊淡入用多层高斯采样近似；文字超宽时 `…` 省略并悬停用 `riff_ui::tooltip::Tooltip` 显示全文 |
| `icon-button` | `icon_button` | base | 完成 | 普通按钮用 gpui-base `Button`，设 `pressed` 时切为 gpui-base `Toggle`（提供 `aria-pressed`）；外观自绘（含 `:active` 95% 缩放、`reduce_motion` 下跳过缩放）。已与原库 35 格（亮/暗 Stacked 与 Overlay）逐格对齐。悬停/聚焦提示已切换为自研 `riff_ui::tooltip::Tooltip` |
| `badge` | `badge` | 自研 | 完成 | 六色调胶囊标签（`neutral` / `info` / `success` / `warning` / `danger` / `purple`），支持 `sm`/`md`、`dot`、`live`（1.8s 脉冲光环，支持冻结帧与 `reduce_motion`）、`icon`、等宽数字 `count`（`max+` 上限与千分位分组）及空值不渲染。已与原库 47 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `kbd` | `kbd` | 自研 | 完成 | 快捷键按键帽（单键与组合键），支持 `sm`/`md`、`mac`/`other` 平台映射（`mod` 在 Mac 为 `⌘`、其余为 `Ctrl`）及 1px 立体底边（`inset -1px`），屏幕阅读器自动拼读完整键名。已与原库 56 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `separator` | `separator` | 自研 | 完成 | 水平/垂直 1px 分隔线，支持居中标签（`12.5px`/`18px` 行高、`12px` 间距与最小线宽、80% 最大宽度折行）、`vertical` 默认 18px 居中及自定义长度、`decorative` 控制 `role="separator"` 语义及方向/名称。已与原库 14 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `spinner` | `spinner` | 自研 | 完成 | 加载旋转指示器（16×16 viewBox，20% 背景环 + 90° 圆角弧），默认 14px、继承 `currentColor`、750ms 线性旋转（`reduce_motion` 下减速为 2.4s 保持旋转），支持自定义速度、`freeze_motion_at` 冻结帧与仅在设 `label` 时暴露 `role="status"`。已与原库 19 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `skeleton` | `skeleton` | 自研 | 完成 | `Skeleton` 占位条（默认 `100%`×`12px`、受 `max-width: 100%` 限制、`circle` 50% 圆角、`line`/`line_height` 行高垂直居中、全局同步 1.6s 扫光渐变且 `reduce_motion` 下停止）与 `Loading` 容器（`role="status"` + `aria-busy` + `150ms` 延迟防闪烁淡入）。已与原库 12 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `avatar` | `avatar` | 自研 | 完成 | `Avatar`（`20/24/32/40`、`circle`/`square`、8 色确定性背景与 `seed`、首字母 `unicode-segmentation` 分词与后缀跳过、匿名轮廓、图片懒加载与 180ms 淡入、角标 `online`/`away`/`busy` 状态点、Agent 对称 3×3 像素徽记及 1.2s 对角波浪闪烁与 `reduce_motion` 停止）与 `AvatarGroup`（1/4 重叠、`+N`/`99+` 溢出徽章与背景镂空环）。已与原库 60 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `card` | `card` | base | 完成 | `Card` / `CardHeader` / `CardContent` / `CardFooter`（浅灰外托盘 `radius_lg+4px` + 内白卡 `radius_lg`、`--riff-shadow-card` 阴影及 160ms hover 抬升、`selected` 选中态、`interactive` 全卡点击与 `pressed` 切换、内部按钮 `block_mouse_except_scroll` 独立响应、语义角色 `Section`/`Article`/`ListItem` 及外圈 2px 焦点环）。同时查清并修复了 `--riff-shadow-card` 底边偏深问题（`paint::drop_shadows` 用抗锯齿路径画 `0.5px` 实心环，避免 GPUI `cover_bounds` 在 1× 下外扩成 1px）。已与原库 14 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `gradient-text` | `gradient_text` | 自研 | 完成 | `GradientText`（继承父级字体/字号/字重/行高/墨色、`100deg` 线性底色与 3 个按关键帧 `ease-in-out` 漂移的 `closest-side` 径向色斑、`swash` 光栅化字形遮罩 + BGRA `RenderImage` 实现 `background-clip: text`、首现 1100ms 从左向右扫入揭示、`reduce_motion` 静止展示、自定义 1–4 色与 `speed`、语义角色 `Span`/`H1`/`H2`/`P`）。已与原库 12 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `tooltip` | `tooltip` | 自研 | 完成 | `Tooltip`（亮/暗反色气泡、自定义双层阴影 + 暗色 1px 外环、`12.5px/18px` 自动换行最大宽度 `240px`、`shortcut` 快捷键帽、四方向与 `start`/`center`/`end` 对齐及自动翻转/视口边缘 `8px` 夹紧、500ms 首次悬停延迟 / 300ms 邻居免延迟无缝切换 / 120ms 移出宽限可停留在气泡上、Tab 键盘聚焦立即打开而点击聚焦不触发、Esc 捕获先关自身不穿透外层、160ms `ease` 渐显与 3px 方向漂移、`reduce_motion` 跳过动效、受控 `open` 与 `on_open_change`）。同时完成共享定位层 `overlay`、替换 `icon_button` 临时 tooltip，以及 `button` 截断省略与悬停全文提示。已与原库 22 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `popover` | `popover` | 自研 | 完成 | `Popover`（`--riff-overlay` 表面、圆角 `16px`、`--riff-shadow-popover`、默认 `padding 16px` / `max_width 360px`、`title`/`description`/`width`/`unstyled`、`side`/`align`/`offset` 视口翻转与边缘夹紧、点击外部/`Escape` 关闭并将焦点交还触发器、`220ms` 渐显与 `4px` 方向漂移；GPUI 侧无 `scale(0.96→1)` 缩放动效及移动端 coarse 命中放大）。已与原库 19 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `hover-card` | `hover_card` | 自研 | 完成 | `HoverCard`（`--riff-overlay` 表面、圆角 `16px`、`--riff-shadow-popover`、默认 `padding 16px` / `max_width 320px`、`side` 默认 `bottom` / `align` 默认 `start` / `offset 8px` / `open_delay 600ms` / `close_delay 300ms`、单窗口仅激活一张、`window.last_input_was_touch()` 触摸不打开、触发器到卡片对角线凸包走廊与 `4px` 容差保持打开、键盘 `:focus-visible` 延迟打开与 `Tab` 顺畅进入卡片内链接、`Escape` 关闭且焦点在内时交还触发器、`200ms` 淡入+`4px` 漂移；GPUI 侧无 `scale(0.97→1)` 缩放动效及移动端 coarse 命中字号调整）。已与原库 13 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `dropdown-menu` | `dropdown_menu` | 自研 | 完成 | `DropdownMenu`（复用内部共享 `menu` 引擎；`--riff-overlay` 表面、圆角 `14px`、`--riff-shadow-popover`、默认宽 `180px..320px` 及长文案省略号截断、图标/品牌 `logo`/次行描述/快捷键/单选与多选对勾及其 `280ms` 描边绘入动效、`180ms` 行间顺滑滑动底亮、危险项、`divider`、`disabled` 跳过、标题/自定义页眉/空态、键盘 `↓`/`↑`/`Home`/`End`/`A–Z` 700ms 前置搜索/`→`/`←` 级联子菜单/`keep_open`/`Esc`/`Tab` 关闭还焦与点击外部关闭；GPUI 侧无面板 `scale(0.96→1)` 缩放及移动端 `40px` 行高调整）。已与原库 12 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `context-menu` | `context_menu` | 自研 | 完成 | `ContextMenu`（与 `DropdownMenu` 共用 `menu` 引擎；在目标区域任意位置右键锚定指针点弹出、支持静态 `items` 与按点动态生成 `items_for_point`、`Shift+F10` / `ContextMenu` 键从焦点锚点打开并聚焦首项、`disabled` 或空菜单时不拦截右键、重新右键刷新坐标、点击外部关闭、选后或 `Esc`/`Tab` 关闭还焦、图标/描述/快捷键/单多选对勾/危险项/级联子菜单/滑动底亮齐全；GPUI 桌面环境未内置触摸 500ms 长按触发，且无面板 `scale(0.96→1)` 缩放）。已与原库 4 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `dialog` | `dialog` | 自研 | 完成 | `Dialog` 与 `AlertDialog`（灰外托盘 `radius_lg+4px` + 内白卡 `radius_lg`、`sm 400 / md 480 / lg 720` 与自定义 `width`、固定头尾与可滚动内容区、关闭按钮带 `Tooltip`、点击遮罩与 `Esc` 经由 `OverlayStack` 按最顶层顺序关闭并交还触发器焦点、`Tab`/`Shift+Tab` 模态焦点循环、初始焦点优先正文→页脚→面板、`AlertDialog` 默认焦点落在安全的 `Cancel` 并拒绝遮罩关闭、`busy` 下禁止 `Esc`/`Cancel`/遮罩关闭、支持 `contained` 局部舞台、窄容器 `≤520px` 自动切为贴底抽屉且 `AlertDialog` 按钮等分撑满；GPUI 侧无面板 `scale(0.97→1)` 缩放，滚动边缘 1px 遮盖渐隐由标准滚动裁剪替代）。已与原库 8 格（亮/暗 Stacked 与 Overlay）对照对齐 |
| `command-menu` | `command_menu` | base | 完成 | `CommandMenu`（搜索框基于 gpui-base `InputState`/`Input`，其余界面与行为自研；灰外托盘 `radius_lg+4px` + 内白卡 `radius_lg` + `--riff-shadow-popover`、默认 `Suggestions`（5 项自带矢量绘制的品牌 Logo 与动态 `Ctrl/⌘+1..5`）/`Share`/`General`/`Go to` 四组、`320ms` 列表高度过渡与 `220ms` 跨行顺滑底亮胶囊、不区分大小写/重音筛选并动态置顶 `Ask assistant about “…”` 与 `<mark>` 蓝底命中高亮、`↑`/`↓` 循环与自动滚入视野、`Tab`/`Shift+Tab` 跨组跳转、`Enter` 执行（含 600ms 轻按回弹与 1800ms 页脚绿色对勾播报）、`Esc` 先清输入再关闭、窗口级 `Ctrl/⌘+K` 聚焦全选、窄容器 `≤480px` 隐藏副标题/快捷键/页脚提示并切为 44px 行高）。已与原库 6 格（亮/暗 Stacked 与 Overlay）对照对齐 |

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
