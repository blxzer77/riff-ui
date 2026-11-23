# riff-ui

基于 [GPUI](https://gpui.rs/) 的 Rust UI 组件库，供 [riff-studio](https://github.com/blxzer77/riff-studio) 的桌面端使用。

目标是用 Rust 复现团队的 React 组件库 riff-uiux：行为复用 gpui-base，结构和样式自己写；跟 shadcn 风格接近的组件直接包装 gpui-component。复现进度见 [`docs/components.md`](docs/components.md)，后续规划见 [`docs/roadmap.md`](docs/roadmap.md)。

## 结构

```
crates/
├── riff-ui/            组件库本体
│   ├── assets/fonts/   Inter（OFL），init 时注册，保证各机器字宽一致
│   ├── src/theme/      设计 token（对应 riff-uiux 的 --riff-* 变量）
│   └── src/button/     按钮
└── riff-story/         组件展示程序
    ├── src/registry.rs     所有组件的登记表
    ├── src/components/     每个组件：用例格子的 GPUI 实现 + 可选演示页
    ├── src/compare.rs      Compare 页
    └── src/shell.rs        侧栏、搜索、工具栏
stories/<slug>.json     每个组件的用例，原库和 GPUI 两边共用
tools/reference/        生成原库对照截图（产物在 target/，不提交）
```

## 依赖

`gpui-kit` 锁在 `=0.10.0`。gpui-kit 每个版本都精确锁定一个 gpui 快照，所以升级时要和 `Cargo.lock` 一起改。

Linux 上编译需要这些系统库（Fedora）：

```sh
sudo dnf install fontconfig-devel freetype-devel libxkbcommon-devel libxkbcommon-x11-devel \
  libxcb-devel libX11-devel wayland-devel vulkan-loader-devel alsa-lib-devel libzstd-devel
```

## 运行

```sh
cargo run                                   # 打开 story，第一个组件的 Compare 页
cargo run -- button                         # 打开某个组件（或 tokens）
cargo run -- button --demo                  # 它的演示页
cargo run -- button --mode overlay --dark   # 指定对照模式、暗色
cargo test --workspace
```

## 对照原库

story 左侧按原库的分类列出组件，可搜索。每个组件有两个页签：

- **Compare**：逐格对照，每格上面是原库截图，下面是实时 GPUI 组件，大小对齐，格子下面写着格子名（反馈差异时用这个名字）。四种模式：上下对照、只看原版、只看 GPUI、叠加（GPUI 半透明盖在原图上）。
- **Demo**：可以自由点的交互演示。

两边用的是同一份用例 `stories/<slug>.json`，按 分区 → 行 → 格 组织，每格是纯数据的 props；悬停、按下、焦点用 `"state"` 固定。

原版截图是 `target/reference/` 下的构建产物，不提交，由 `tools/reference/build.py` 生成：用原库自己的组件和 tokens 渲染用例，headless Chrome 以 2x 截图后按格裁开。需要原库检出（只读）、原库 gallery 里的 `esbuild`、原库 node_modules 里的 React、`node` 和 `google-chrome`。

```sh
python3 tools/reference/build.py            # 只重新生成过期的组件
python3 tools/reference/build.py button     # 指定组件，无论是否过期
python3 tools/reference/build.py --check    # 列出过期的组件，有则退出码 1
```

截图记录了用例、适配器、原库组件、tokens、字体的哈希；任何一个变了就算过期。Compare 页工具栏显示截图状态（最新 / 过期 / 缺失），过期或缺失时给出生成命令，生成后点「Reload pictures」即可，不用重启。

新增组件的步骤见 [`AGENTS.md`](AGENTS.md)。
