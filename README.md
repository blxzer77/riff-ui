# riff-ui

基于 [GPUI](https://gpui.rs/) 的 Rust UI 组件库，供 [riff-studio](https://github.com/blxzer77/riff-studio) 的桌面端使用。

目标是用 Rust 复现团队的 React 组件库 riff-uiux：行为复用 gpui-base，结构和样式自己写；跟 shadcn 风格接近的组件直接包装 gpui-component。复现进度见 [`docs/components.md`](docs/components.md)。

## 结构

```
crates/
├── riff-ui/      组件库本体
│   └── src/theme/  设计 token（对应 riff-uiux 的 --riff-* 变量）
└── riff-story/   组件展示程序
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
cargo run                         # 打开 story
RIFF_STORY_MODE=dark cargo run    # 以暗色启动
cargo test -p riff-ui
```
