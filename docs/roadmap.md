# 路线图

接下来做什么、按什么顺序。已完成的组件和进度见 [`components.md`](components.md)，每个组件的做法见 [`AGENTS.md`](../AGENTS.md)。

## 范围

原库 riff-uiux 共 220 个组件，分 25 类。

- **不做（44 个）**：App screens 的 43 个屏幕级组件（`settings-*`、`crm`、`agent-chat`、`workflow-builder` 等），以及 `toaster`。它们残留 `X$1` 这类压缩名、依赖 `src/internal/core.tsx`，不能当规格用。riff-ui 只放组件库，屏幕由应用自己用组件拼。
- **要做（176 个）**：其余各类。原库的组件文件互不引用，每个都是独立的，所以先后顺序看的是 gpui-base 已有哪些行为，以及组件之间共用的样式和交互模式。

## 已知遗留

| 事项 | 来源 | 处理 |
| ---- | ---- | ---- |
| button 文字被截断时，悬停显示全文 | button | 做完 tooltip 后补 |
| `--riff-font-mono` / `--riff-mono` 未实现，用的是 gpui-component 的等宽字体 | theme | 做 code-block、kbd 时一并处理 |
| 卡片阴影 `--riff-shadow-card` 紧贴底边那一行比原库深（225 对 237） | theme | 做 card 时对照查清 |
| 按下状态的路径抗锯齿在显卡不支持 4× MSAA 时会降到 2× 或 1×，只在本机验证过 | button | 换机器时复查 |
| `tools/reference/build.py` 写死了原库路径和 cargo 缓存里 Lucide 图标的路径，只能在这台机器上跑 | 工具 | 需要换机器时改成可配置 |

## 建议顺序

每一阶段先做的那个组件会定下这一类的写法，后面照着做。

### 阶段 1：小件（9 个）

把 button 定下的写法推广到更多组件，风险低，也是后面各阶段都会用到的部件。

~~`icon-button`~~（已完成）、~~`badge`~~（已完成）、~~`kbd`~~（已完成）、~~`separator`~~（已完成）、~~`spinner`~~（已完成）、`skeleton`、`avatar`、`card`、`gradient-text`

- `icon-button`、`badge`、`kbd`、`separator`、`spinner` 已完成；下一个做 `skeleton`。
- `card` 用到 `--riff-shadow-card`，顺带查清上面的阴影遗留。
- gpui-base 有 `avatar` 的行为。

### 阶段 2：浮层（7 个）

共用一套浮层定位和出入场动画，先定下这套写法。

`tooltip`、`popover`、`hover-card`、`dropdown-menu`、`context-menu`、`dialog`、`command-menu`

- gpui-base 有 `tooltip`、`popover`、`hover_card`、`dialog`、`positioner`。
- `tooltip` 做完回头补 button 的截断全文。

### 阶段 3：表单（约 20 个）

共用输入框的外观、焦点环、错误状态和 `form-field` 的标签与说明文字。

`text-field`、`text-area`、`form-field`、`checkbox`、`switch`、`segmented-control`、`toggle-group`、`select`、`combobox`、`number-input`、`otp-input`、`tag-input`、`rating`、`date-picker`、`date-range-picker`、`time-picker`、`color-picker`、`dropzone`、`filter-menu`、`stepper-form`

- gpui-base 有 `input`、`checkbox`、`switch`、`select`、`combobox`、`number_input`、`otp_input`、`toggle_group`、`date_picker`、`time_field`、`color_picker`。
- `select`、`combobox`、`filter-menu` 依赖阶段 2 的浮层。

### 阶段 4：导航与数据展示

`breadcrumb`、`pagination`、`scroll-area`、`resizable`、`tree-view`、`data-table`、`empty-state`、`progress-tracker`、`notification-stack`、`usage-meter`、`code-block`、`profile-menu`

- gpui-base 有 `pagination`、`scrollbar`、`resizable`、`tree`、`table`、`progress`。
- `code-block` 处理等宽字体遗留。

### 阶段 5：复合组件

Chat（10）、Cards 其余、Agents（55）、图表（`kpi-chart`、`trend-chart`、`breakdown-chart` 等）、Card illustrations（17）、Product scenes（7），以及其余单个的展示类组件。

这些组件由前几阶段的部件组合而成，进入阶段 5 前再按实际需要细排。其中一部分 Agents 和图表组件带压缩名（`agent-fleet`、`stat-card` 等），做之前要逐个确认能不能当规格用。
