# 更新日志

本站记录 ohmic 的已发布版本；完整提交历史见 [GitHub](https://github.com/asionbo/ohmic/commits/main)，英文版清单见仓库根目录的 [CHANGELOG.md](https://github.com/asionbo/ohmic/blob/main/CHANGELOG.md)。

## [v0.1.8](https://github.com/asionbo/ohmic/releases/tag/v0.1.8) · 2026-11-17

- 修复网络变化后二维码链接过期的问题：此前 LAN IP 只在启动时检测一次，一夜之间 DHCP 换了 IP、切换 Wi-Fi 或睡眠唤醒后，二维码与「复制手机链接」仍指向旧地址，且 `Host` 校验也只认旧 IP，手机完全连不上。现在：
  - 后台线程每 30 秒重新枚举局域网地址；
  - 选中的 IP 失效时自动切换到最优候选（手动在「网络」菜单选过的地址只要仍在线就优先保留）；
  - 打开配对页时先刷新一次网络，保证二维码始终反映当前地址；
  - 网卡列表变化时自动重建托盘「网络」子菜单，并在托盘状态栏提示「本机地址已变，请重新扫码」；
  - `Host` 校验放宽为接受候选列表中的任意 IP。

## [v0.1.7](https://github.com/asionbo/ohmic/releases/tag/v0.1.7) · 2026-11-12

- 新增结构化文件日志（macOS 位于 `~/Library/Caches/ohmic/logs/ohmic.log`），GUI 进程 stderr 不可用时的致命错误也会落盘。
- 新增看门狗线程：每 20 秒探测主 runloop，连续约 60 秒无心跳即以退出码 97 退出（交由 launchd / 用户重启），杜绝主线程死循环导致的无声挂死。

## [v0.1.6](https://github.com/asionbo/ohmic/releases/tag/v0.1.6) · 2026-11-11

- 托盘菜单新增 GitHub 仓库入口与版本号显示。

## [v0.1.5](https://github.com/asionbo/ohmic/releases/tag/v0.1.5) · 2026-11-11

- 项目由 **nomic** 更名为 **ohmic**。
- 升级 GitHub Actions runner 版本（macOS 15）。

## [v0.1.4](https://github.com/asionbo/ohmic/releases/tag/v0.1.4) · 2026-11-06

- 更新托盘图标。

## [v0.1.3](https://github.com/asionbo/ohmic/releases/tag/v0.1.3) · 2026-11-06

- 托盘菜单新增「粘贴后自动回车」开关（默认关闭），开启后粘贴成功会追加模拟回车键。

## [v0.1.2](https://github.com/asionbo/ohmic/releases/tag/v0.1.2) · 2026-11-05

- 新增 VitePress 文档站（本站）。
- macOS 事件循环显式使用 `Accessory` 激活策略：托盘常驻菜单栏，不再出现在 Dock 中。
- 修复若干 lint 问题。

## [v0.1.1](https://github.com/asionbo/ohmic/releases/tag/v0.1.1) · 2026-11-05

- 新增中文 README 与中英文界面切换（托盘菜单或 `--lang zh`），默认仍为英文。
- 托盘图标改为高分辨率绘制，并加入 macOS 应用图标生成脚本。
- 发布流程加入签名、公证与多平台打包，同时完善 README、`.gitignore` 等文档。
- 修复若干界面与注入逻辑的小问题。

## [v0.1.0](https://github.com/asionbo/ohmic/releases/tag/v0.1.0) · 2026-11-05

- 首个公开版本：手机扫码 → 局域网 WebSocket → 剪贴板 → 模拟 ⌘/Ctrl+V 的完整链路，macOS 已验证。
- 加入每次启动重新生成的令牌、Host / Origin 校验、连接数与消息大小限制。
- 内嵌二维码 SVG 与手机输入页，内置中英文界面文案，页面使用严格 CSP。
- 加入 GitHub Actions 的 lint 与 release 工作流，并补充 GPL-3.0 许可证。
