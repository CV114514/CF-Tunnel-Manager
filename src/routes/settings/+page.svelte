<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "$lib/api";
  import type { VersionInfo } from "$lib/types";

  let exePath = $state("");
  let version = $state<VersionInfo | null>(null);
  let busy = $state(false);
  let msg = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);

  onMount(async () => {
    const state = await api.getState();
    exePath = state.settings.cloudflared_exe_path ?? "";
    if (exePath) {
      await testVersion();
    }
  });

  async function pickFile() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "cloudflared", extensions: ["exe"] }],
      });
      if (typeof selected === "string") {
        exePath = selected;
      }
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function save() {
    if (!exePath) {
      errorMsg = "请先选择 cloudflared.exe 路径";
      return;
    }
    busy = true;
    errorMsg = null;
    msg = null;
    try {
      version = await api.setCloudflaredExe(exePath);
      msg = `已保存。检测到版本：${version.version}`;
    } catch (e) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function testVersion() {
    busy = true;
    errorMsg = null;
    try {
      version = await api.testCloudflaredVersion();
      msg = `版本：${version.version}`;
    } catch (e) {
      errorMsg = String(e);
      version = null;
    } finally {
      busy = false;
    }
  }

  async function openCloudflaredDir() {
    try {
      await api.openCloudflaredDir();
    } catch (e) {
      errorMsg = String(e);
    }
  }
</script>

<h1>设置</h1>

<div class="card">
  <h2>cloudflared 可执行文件</h2>
  <p class="hint">
    本工具不打包 cloudflared，需自行下载。下载地址：
    <a href="https://github.com/cloudflare/cloudflared/releases/latest" target="_blank" rel="noopener">
      cloudflared-windows-amd64.exe
    </a>
  </p>

  <div class="field">
    <label for="exe">exe 路径</label>
    <div class="row">
      <input id="exe" bind:value={exePath} placeholder="D:\tool\cloudflare\cloudflared-windows-amd64.exe" />
      <button class="btn fixed-btn" onclick={pickFile}>浏览…</button>
      <button class="btn primary fixed-btn" onclick={save} disabled={busy}>保存并检测</button>
    </div>
  </div>

  {#if version}
    <div class="version-box">
      <span class="dot ok"></span>
      <div>
        <div class="version-label">已检测</div>
        <div class="version-value">{version.version}</div>
        <div class="version-raw">{version.raw}</div>
      </div>
      <button class="btn fixed-btn" onclick={testVersion} disabled={busy} style="margin-left: auto;">重新检测</button>
    </div>
  {/if}

  {#if msg}
    <div class="msg ok">{msg}</div>
  {/if}
  {#if errorMsg}
    <div class="msg err">{errorMsg}</div>
  {/if}
</div>

<div class="card">
  <h2>cloudflared 主目录</h2>
  <p class="hint">cert.pem 和隧道凭证 JSON 默认存在 <code>%USERPROFILE%\.cloudflared\</code></p>
  <button class="btn" onclick={openCloudflaredDir}>打开 .cloudflared 目录</button>
</div>

<style>
  .hint {
    color: #a6adc8;
    font-size: 13px;
    margin: 8px 0 16px;
  }
  .row {
    flex-wrap: nowrap;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  .fixed-btn {
    flex: 0 0 auto;
    white-space: nowrap;
    min-width: 96px;
  }
  .version-box {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: #313244;
    border-radius: 6px;
    margin-top: 12px;
  }
  .version-label {
    font-size: 11px;
    color: #a6adc8;
    text-transform: uppercase;
  }
  .version-value {
    font-size: 18px;
    font-weight: 700;
    color: #a6e3a1;
  }
  .version-raw {
    font-size: 11px;
    color: #6c7086;
    margin-top: 2px;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: inline-block;
  }
  .dot.ok {
    background: #a6e3a1;
  }
  .msg {
    margin-top: 12px;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
  }
  .msg.ok {
    background: #a6e3a120;
    color: #a6e3a1;
    border: 1px solid #a6e3a140;
  }
  .msg.err {
    background: #f38ba820;
    color: #f38ba8;
    border: 1px solid #f38ba840;
  }
  code {
    background: #313244;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 12px;
  }
</style>
