<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { onLoginCertChanged } from "$lib/events";
  import type { CertInfo } from "$lib/types";

  let cert = $state<CertInfo | null>(null);
  let busy = $state(false);
  let msg = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);

  onMount(async () => {
    await refresh();
    const off = await onLoginCertChanged(refresh);
    return off;
  });

  async function refresh() {
    try {
      cert = await api.checkLogin();
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function login() {
    busy = true;
    msg = null;
    errorMsg = null;
    try {
      const ok = await api.startLogin();
      if (ok) {
        msg = "登录成功，cert.pem 已写入。";
      } else {
        errorMsg = "登录超时（5 分钟内未检测到 cert.pem）。";
      }
      await refresh();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<h1>Cloudflare 登录</h1>

<div class="card">
  {#if cert?.logged_in}
    <div class="status-box ok">
      <span class="dot ok"></span>
      <div>
        <div class="status-label">已登录</div>
        <div class="status-detail">cert.pem 路径：<code>{cert?.path}</code></div>
        {#if cert?.expires_at}
          <div class="status-detail">最后修改：{cert.expires_at}</div>
        {/if}
      </div>
    </div>
    <p class="hint">
      登录态由 cloudflared 自己管理。退出登录 = 删除 cert.pem（可在「设置」页打开 .cloudflared 目录手动删除）。
    </p>
  {:else}
    <div class="status-box warn">
      <span class="dot warn"></span>
      <div>
        <div class="status-label">未登录</div>
        <div class="status-detail">点击下方按钮会打开浏览器走 Cloudflare OAuth 流程。</div>
      </div>
    </div>
  {/if}

  <div style="margin-top: 16px;">
    <button class="btn primary login-btn" onclick={login} disabled={busy}>
      {busy ? "等待登录完成…" : (cert?.logged_in ? "重新登录" : "Login with Cloudflare")}
    </button>
  </div>

  {#if msg}
    <div class="msg ok">{msg}</div>
  {/if}
  {#if errorMsg}
    <div class="msg err">{errorMsg}</div>
  {/if}
</div>

<style>
  .status-box {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px;
    border-radius: 6px;
    margin-bottom: 12px;
  }
  .status-box.ok {
    background: #a6e3a120;
    border: 1px solid #a6e3a140;
  }
  .status-box.warn {
    background: #f9e2af20;
    border: 1px solid #f9e2af60;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    margin-top: 5px;
    display: inline-block;
  }
  .dot.ok {
    background: #a6e3a1;
  }
  .dot.warn {
    background: #f9e2af;
  }
  .status-label {
    font-weight: 600;
    font-size: 15px;
    margin-bottom: 4px;
  }
  .status-detail {
    font-size: 12px;
    color: #a6adc8;
    margin-top: 2px;
  }
  .hint {
    color: #a6adc8;
    font-size: 12px;
    margin-top: 12px;
  }
  code {
    background: #313244;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 12px;
    word-break: break-all;
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
  .login-btn {
    min-width: 180px;
  }
</style>
