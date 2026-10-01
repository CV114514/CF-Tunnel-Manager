# CF Tunnel Manager

一个 Windows 桌面 GUI 工具，把本机端口通过 [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) 暴露到公网，绑定你自己 Cloudflare 账号下的域名。不用公网 IP、不用端口转发、不用动路由器，HTTPS 由 Cloudflare 自动提供。

![Tauri](https://img.shields.io/badge/Tauri-2-orange) ![Svelte](https://img.shields.io/badge/Svelte-5-ff3e00) ![Rust](https://img.shields.io/badge/Rust-stable-dea584) ![Platform](https://img.shields.io/badge/Platform-Windows-blue)

## 为什么做这个

cloudflared 是命令行工具，建隧道要手敲 `tunnel login` → `tunnel create` → 写 `config.yml` → `tunnel route dns` → `tunnel run`，还要自己管进程生命周期、看日志、防孤儿进程。这个工具把这些全包成可视化界面：填域名、填端口、点启动，托盘常驻。

## 功能

- **OAuth 登录**：一键调起浏览器走 Cloudflare OAuth，自动检测 `cert.pem` 写入
- **隧道管理**：新建 / 编辑 / 删除 Named Tunnel，自动调 `tunnel create` + `route dns`
- **一键启停**：点按钮启动 `tunnel run`，状态实时显示（Starting → Connected）
- **实时日志**：cloudflared 子进程的 stdout/stderr 流式推送，按级别高亮
- **Metrics 轮询**：自动发现 metrics 端口，定时拉 Prometheus 指标（活跃连接数、edge 位置）
- **托盘常驻**：关窗不退出，托盘图标颜色聚合状态（绿/黄/红/灰）
- **孤儿进程清理**：Windows Job Object 绑定子进程 + PID 文件双保险，GUI 崩溃也自动 kill cloudflared
- **多隧道并发**：每隧道独立 config.yml + 随机 metrics 端口，互不冲突

## 截图

| 概览 | 隧道列表 | 实时日志 |
|---|---|---|
| ![overview](docs/overview.png) | ![tunnels](docs/tunnels.png) | ![logs](docs/logs.png) |

## 安装

### 方式 1：下载预编译版

到 [Releases](../../releases) 下载 `CF-Tunnel-Manager_0.1.0_x64-setup.exe`，双击安装。

### 方式 2：自己编译

需要：
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/)（推荐 GNU 工具链）
- [cloudflared](https://github.com/cloudflare/cloudflared/releases/latest)（运行时需要，不打包）

```bash
git clone https://github.com/<your-username>/cf-tunnel-manager.git
cd cf-tunnel-manager
npm install
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/`。

## 使用流程

1. **下载 cloudflared**：[cloudflared-windows-amd64.exe](https://github.com/cloudflare/cloudflared/releases/latest)，放到任意目录
2. **打开本工具**：设置页填 cloudflared.exe 路径，自动检测版本
3. **登录 Cloudflare**：登录页点按钮，浏览器走 OAuth，完成后 `~/.cloudflared/cert.pem` 自动写入
4. **新建隧道**：填隧道名（小写+连字符）、公网域名（你 CF 账号下托管的域名）、本地服务 URL（如 `http://localhost:8080`）
5. **启动**：点启动按钮，状态变绿表示已连上 edge
6. **访问**：浏览器打开你的域名，公网 HTTPS 直达本机服务

## 工作原理

```
用户浏览器 ──HTTPS──> Cloudflare Edge ──tunnel──> cloudflared(本机) ──> localhost:8080
```

cloudflared 主动出站连 Cloudflare edge，不需要公网 IP 或端口转发。本工具 spawn cloudflared 子进程，解析它的 stdout/stderr 实时日志来追踪状态（Starting → Connected）、发现 metrics 端口、提取 edge 连接数。

## 技术栈

- **Tauri 2**：Rust 后端 + WebView 前端，打包体积小（~10MB）
- **Svelte 5 + SvelteKit**：前端 UI，runes 响应式
- **Rust**：tokio 异步，serde 序列化，regex 解析日志，win32job 绑进程
- **cloudflared CLI**：所有 Cloudflare 操作走 shell out，不直接调 API

## 项目结构

```
cf-tunnel-manager/
├── src/                          # Svelte 前端
│   ├── routes/
│   │   ├── +page.svelte          # 概览
│   │   ├── tunnels/+page.svelte  # 隧道管理 + 日志面板
│   │   ├── login/+page.svelte    # OAuth 登录
│   │   └── settings/+page.svelte # cloudflared 路径设置
│   └── lib/                      # API 封装、事件监听、类型定义
└── src-tauri/
    ├── Cargo.toml
    └── src/
        ├── lib.rs                # Tauri Builder + setup
        ├── commands.rs           # 所有 #[tauri::command]
        ├── state.rs              # 持久化 + 运行时状态
        ├── tray.rs               # 托盘图标 + 状态聚合
        ├── pid_file.rs           # PID 文件 + 孤儿扫
        ├── fs_paths.rs           # 路径助手
        └── cloudflared/
            ├── mod.rs            # 版本探测
            ├── login.rs          # OAuth 流程
            ├── tunnel.rs         # create/route/delete/list
            ├── run.rs            # spawn tunnel run + 日志流 + Job Object
            ├── config.rs         # config.yml 读写
            └── metrics.rs        # Prometheus 指标解析
```

## 限制

- 仅支持 Windows（用了 win32job 和 Windows API）
- 隧道状态靠解析 cloudflared 日志推断，不调 Cloudflare API（CLI 措辞变更可能影响状态识别）
- 不支持远程隧道列表的反向同步（本地 profile 是源）
- 不打包 cloudflared，需用户自行下载

## License

MIT
