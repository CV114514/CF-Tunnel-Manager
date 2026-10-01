<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { onTunnelLog, onTunnelStatus } from "$lib/events";
  import type { AppStateSnapshot, LogLine, TunnelStatus } from "$lib/types";
  import { statusText, statusColor, isRunning } from "$lib/status";

  let snapshot = $state<AppStateSnapshot | null>(null);
  let showEditor = $state(false);
  let editingId = $state<string | null>(null);
  let logsTunnelId = $state<string | null>(null);
  let logs = $state<LogLine[]>([]);
  let logsStatus = $state<TunnelStatus | null>(null);
  let busyId = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);

  let newName = $state("");
  let newHostname = $state("");
  let newUrl = $state("http://localhost:");

  onMount(async () => {
    await refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  });

  async function refresh() {
    try {
      snapshot = await api.getState();
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function openEditor(id?: string) {
    if (id) {
      const t = snapshot?.tunnels.find((x) => x.id === id);
      if (t) {
        editingId = id;
        newName = t.name;
        newHostname = t.hostname;
        newUrl = t.target_service_url;
      }
    } else {
      editingId = null;
      newName = "";
      newHostname = "";
      newUrl = "http://localhost:";
    }
    showEditor = true;
  }

  async function saveTunnel() {
    errorMsg = null;
    try {
      if (editingId) {
        await api.editTunnel(editingId, {
          hostname: newHostname,
          target_service_url: newUrl,
        });
      } else {
        await api.createTunnel({
          name: newName,
          hostname: newHostname,
          target_service_url: newUrl,
        });
      }
      showEditor = false;
      await refresh();
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function startTunnel(id: string) {
    busyId = id;
    errorMsg = null;
    try {
      await api.startTunnel(id);
      await refresh();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      busyId = null;
    }
  }

  async function stopTunnel(id: string) {
    busyId = id;
    errorMsg = null;
    try {
      await api.stopTunnel(id);
      await refresh();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      busyId = null;
    }
  }

  async function deleteTunnel(id: string) {
    if (!confirm("确定删除该隧道？此操作会在 Cloudflare 账号里删除远端隧道和 DNS 路由。")) {
      return;
    }
    busyId = id;
    errorMsg = null;
    try {
      await api.deleteTunnel(id);
      await refresh();
    } catch (e) {
      errorMsg = String(e);
    } finally {
      busyId = null;
    }
  }

  async function viewLogs(id: string) {
    logsTunnelId = id;
    logs = [];
    logsStatus = null;
    try {
      logs = await api.getTunnelLogs(id);
      logsStatus = await api.getTunnelStatus(id);
      const off1 = await onTunnelLog(id, (line) => {
        logs.push(line);
        if (logs.length > 5000) logs = logs.slice(-5000);
      });
      const off2 = await onTunnelStatus(id, (s) => {
        logsStatus = s;
      });
      const close = () => {
        off1();
        off2();
      };
      (window as any).__logsClose = close;
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function closeLogs() {
    if ((window as any).__logsClose) {
      (window as any).__logsClose();
      (window as any).__logsClose = null;
    }
    logsTunnelId = null;
    logs = [];
  }

  let logsTunnel = $derived(
    logsTunnelId ? snapshot?.tunnels.find((t) => t.id === logsTunnelId) : null
  );
</script>

<h1>隧道</h1>

<div class="card" style="padding: 0;">
  {#if !snapshot || snapshot.tunnels.length === 0}
    <div style="padding: 40px; text-align: center; color: #a6adc8;">
      暂无隧道。<button class="btn primary" onclick={() => openEditor()}>新建隧道</button>
    </div>
  {:else}
    <table>
      <thead>
        <tr>
          <th>名称</th>
          <th>域名</th>
          <th>本地服务</th>
          <th>DNS</th>
          <th>状态</th>
          <th>操作</th>
        </tr>
      </thead>
      <tbody>
        {#each snapshot.tunnels as t}
          <tr>
            <td>{t.name}</td>
            <td><code>{t.hostname}</code></td>
            <td><code>{t.target_service_url}</code></td>
            <td>
              {#if t.dns_routed}
                <span class="dot ok"></span>
              {:else}
                <span class="dot warn"></span>
              {/if}
            </td>
            <td>
              <span class="dot" style="background: {statusColor(snapshot.running[t.id])}"></span>
              {statusText(snapshot.running[t.id])}
            </td>
            <td class="actions">
              {#if isRunning(snapshot.running[t.id])}
                <button class="btn action-btn" onclick={() => stopTunnel(t.id)} disabled={busyId === t.id}>停止</button>
                <button class="btn action-btn" onclick={() => viewLogs(t.id)}>日志</button>
              {:else}
                <button class="btn primary action-btn" onclick={() => startTunnel(t.id)} disabled={busyId === t.id}>启动</button>
                <button class="btn action-btn" onclick={() => openEditor(t.id)}>编辑</button>
                <button class="btn danger action-btn" onclick={() => deleteTunnel(t.id)} disabled={busyId === t.id}>删除</button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<button class="btn primary" onclick={() => openEditor()} style="margin-top: 12px;">+ 新建隧道</button>

{#if showEditor}
  <div class="modal-backdrop" onclick={() => (showEditor = false)}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{editingId ? "编辑隧道" : "新建隧道"}</h3>
      <div class="field">
        <label>隧道名称</label>
        <input bind:value={newName} placeholder="my-tunnel" disabled={!!editingId} />
        <div class="hint">小写字母、数字、连字符；不可改名</div>
      </div>
      <div class="field">
        <label>公网域名</label>
        <input bind:value={newHostname} placeholder="app.example.com" />
        <div class="hint">必须是你 Cloudflare 账号下已托管的域名</div>
      </div>
      <div class="field">
        <label>本地服务 URL</label>
        <input bind:value={newUrl} placeholder="http://localhost:3000" />
        <div class="hint">cloudflared 会把流量代理到这里</div>
      </div>
      {#if errorMsg}
        <div class="msg err">{errorMsg}</div>
      {/if}
      <div class="modal-actions">
        <button class="btn modal-btn" onclick={() => (showEditor = false)}>取消</button>
        <button class="btn primary modal-btn" onclick={saveTunnel}>{editingId ? "保存" : "创建"}</button>
      </div>
    </div>
  </div>
{/if}

{#if logsTunnelId}
  <div class="modal-backdrop" onclick={closeLogs}>
    <div class="modal log-modal" onclick={(e) => e.stopPropagation()}>
      <div class="log-header">
        <div>
          <h3 style="margin: 0;">{logsTunnel?.name} — 实时日志</h3>
          <div class="log-sub">
            <span class="dot" style="background: {statusColor(logsStatus ?? undefined)}"></span>
            {statusText(logsStatus ?? undefined)}
          </div>
        </div>
        <button class="btn" onclick={closeLogs}>关闭</button>
      </div>
      <div class="log-body">
        {#each logs as line}
          <div class="log-line" data-level={line.level}>
            <span class="log-ts">{new Date(line.ts).toLocaleTimeString()}</span>
            <span class="log-level">{line.level}</span>
            <span class="log-source">{line.source}</span>
            <span class="log-text">{line.line}</span>
          </div>
        {/each}
        {#if logs.length === 0}
          <div class="log-empty">暂无日志。启动隧道后会实时显示。</div>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if errorMsg && !showEditor}
  <div class="toast err">{errorMsg}</div>
{/if}

<style>
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: nowrap;
  }
  .action-btn {
    flex: 0 0 auto;
    white-space: nowrap;
    min-width: 56px;
  }
  .modal-btn {
    min-width: 88px;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    vertical-align: middle;
    margin-right: 4px;
  }
  .dot.ok {
    background: #a6e3a1;
  }
  .dot.warn {
    background: #f9e2af;
  }
  code {
    background: #313244;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 12px;
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
    width: 500px;
    max-width: 90vw;
    max-height: 90vh;
    overflow: auto;
  }
  .modal-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }
  .hint {
    font-size: 11px;
    color: #6c7086;
    margin-top: 4px;
  }
  .msg.err {
    background: #f38ba820;
    color: #f38ba8;
    border: 1px solid #f38ba840;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
    margin-top: 12px;
  }

  .log-modal {
    width: 900px;
    max-width: 95vw;
    height: 80vh;
    display: flex;
    flex-direction: column;
    padding: 16px;
  }
  .log-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding-bottom: 12px;
    border-bottom: 1px solid #313244;
    margin-bottom: 12px;
  }
  .log-sub {
    font-size: 13px;
    color: #a6adc8;
    margin-top: 4px;
  }
  .log-body {
    flex: 1;
    overflow-y: auto;
    font-family: "JetBrains Mono", "Consolas", monospace;
    font-size: 12px;
    background: #11111b;
    padding: 8px;
    border-radius: 6px;
  }
  .log-line {
    display: grid;
    grid-template-columns: 80px 50px 60px 1fr;
    gap: 8px;
    padding: 2px 0;
    border-bottom: 1px solid #1e1e2e;
  }
  .log-line[data-level="error"] {
    background: #f38ba820;
  }
  .log-ts {
    color: #6c7086;
  }
  .log-level {
    color: #89b4fa;
    text-transform: uppercase;
    font-weight: 600;
  }
  .log-line[data-level="error"] .log-level {
    color: #f38ba8;
  }
  .log-line[data-level="warn"] .log-level {
    color: #f9e2af;
  }
  .log-source {
    color: #6c7086;
  }
  .log-text {
    color: #cdd6f4;
    word-break: break-all;
    white-space: pre-wrap;
  }
  .log-empty {
    color: #6c7086;
    text-align: center;
    padding: 40px;
  }

  .toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    padding: 10px 20px;
    border-radius: 8px;
    z-index: 1000;
  }
  .toast.err {
    background: #f38ba8;
    color: #1e1e2e;
  }
</style>
