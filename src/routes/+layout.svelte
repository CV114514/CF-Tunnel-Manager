<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { api } from "$lib/api";
  import {
    onGlobalStateChanged,
    onOrphansDetected,
    onWindowHidden,
  } from "$lib/events";
  import type { AppStateSnapshot, PidEntry } from "$lib/types";

  let { children } = $props();

  let snapshot = $state<AppStateSnapshot | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showHiddenToast = $state(false);
  let orphans = $state<PidEntry[]>([]);

  onMount(async () => {
    await refresh();
    loading = false;
    const off1 = await onGlobalStateChanged(refresh);
    const off2 = await onWindowHidden(() => {
      showHiddenToast = true;
      setTimeout(() => (showHiddenToast = false), 3000);
    });
    const off3 = await onOrphansDetected((o) => {
      orphans = o;
    });
    return () => {
      off1();
      off2();
      off3();
    };
  });

  async function refresh() {
    try {
      snapshot = await api.getState();
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function killOrphans() {
    try {
      await api.cleanupOrphans();
      orphans = [];
    } catch (e) {
      error = String(e);
    }
  }

  function exeReady() {
    return snapshot?.settings?.cloudflared_exe_path != null;
  }

  let path = $derived($page.url.pathname);
</script>

<div class="app-shell">
  <nav class="sidebar">
    <div class="brand">
      <span class="logo-dot"></span>
      <span>CF Tunnel</span>
    </div>
    <a href="/" class="nav-item" class:active={path === "/"}>概览</a>
    <a href="/tunnels" class="nav-item" class:active={path.startsWith("/tunnels")}
      >隧道</a
    >
    <a href="/login" class="nav-item" class:active={path === "/login"}>登录</a>
    <a href="/settings" class="nav-item" class:active={path === "/settings"}
      >设置</a
    >
    <div class="nav-status">
      {#if snapshot?.cert?.logged_in}
        <span class="dot ok"></span> 已登录
      {:else}
        <span class="dot warn"></span> 未登录
      {/if}
      <br />
      {#if exeReady()}
        <span class="dot ok"></span> exe 已就绪
      {:else}
        <span class="dot warn"></span> exe 未设置
      {/if}
    </div>
  </nav>

  <main class="content">
    {#if loading}
      <div class="loading">加载中…</div>
    {:else if !exeReady() && path !== "/settings"}
      <div class="banner">
        ⚠️ 未设置 cloudflared.exe 路径，请先到 <a href="/settings">设置</a> 页配置。
      </div>
      {@render children?.()}
    {:else}
      {@render children?.()}
    {/if}
  </main>
</div>

{#if showHiddenToast}
  <div class="toast">窗口已隐藏到托盘，点击托盘图标可恢复。</div>
{/if}

{#if orphans.length > 0}
  <div class="modal-backdrop">
    <div class="modal">
      <h3>发现 {orphans.length} 个遗留的 cloudflared 进程</h3>
      <p>这些进程来自上次未正常退出的会话：</p>
      <ul>
        {#each orphans as o}
          <li>{o.tunnel_name} (PID {o.pid})</li>
        {/each}
      </ul>
      <div class="modal-actions">
        <button class="btn primary modal-btn" onclick={killOrphans}>全部清理</button>
        <button class="btn modal-btn" onclick={() => (orphans = [])}>忽略</button>
      </div>
    </div>
  </div>
{/if}

{#if error}
  <div class="toast error">{error}</div>
{/if}

<style>
  .app-shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100vh;
  }
  .sidebar {
    background: #181825;
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-right: 1px solid #313244;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px 16px;
    font-weight: 700;
    font-size: 16px;
  }
  .logo-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: linear-gradient(135deg, #f6821f, #faae40);
    box-shadow: 0 0 8px #f6821f80;
  }
  .nav-item {
    padding: 8px 12px;
    border-radius: 6px;
    color: #cdd6f4;
    font-size: 14px;
  }
  .nav-item:hover {
    background: #313244;
    text-decoration: none;
  }
  .nav-item.active {
    background: #313244;
    color: #89b4fa;
  }
  .nav-status {
    margin-top: auto;
    padding: 12px;
    font-size: 12px;
    color: #a6adc8;
    line-height: 1.8;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-right: 4px;
    vertical-align: middle;
  }
  .dot.ok {
    background: #a6e3a1;
  }
  .dot.warn {
    background: #f9e2af;
  }

  .content {
    padding: 24px 32px;
    overflow-y: auto;
  }

  .banner {
    background: #f9e2af20;
    border: 1px solid #f9e2af60;
    color: #f9e2af;
    padding: 12px 16px;
    border-radius: 8px;
    margin-bottom: 20px;
  }

  .loading {
    padding: 60px;
    text-align: center;
    color: #a6adc8;
  }

  .toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: #313244;
    color: #cdd6f4;
    padding: 10px 20px;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    z-index: 1000;
  }
  .toast.error {
    background: #f38ba8;
    color: #1e1e2e;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1001;
  }
  .modal {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 12px;
    padding: 24px;
    width: 460px;
    max-width: 90vw;
  }
  .modal h3 {
    margin-top: 0;
  }
  .modal-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }
  .modal-btn {
    min-width: 88px;
  }
</style>

