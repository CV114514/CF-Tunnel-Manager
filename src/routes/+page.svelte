<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import type { AppStateSnapshot } from "$lib/types";
  import { statusText, statusColor, isRunning } from "$lib/status";

  let snapshot = $state<AppStateSnapshot | null>(null);

  onMount(async () => {
    snapshot = await api.getState();
    const interval = setInterval(async () => {
      snapshot = await api.getState();
    }, 5000);
    return () => clearInterval(interval);
  });

  let runningCount = $derived(
    snapshot ? Object.values(snapshot.running).filter(isRunning).length : 0
  );
  let totalTunnels = $derived(snapshot?.tunnels.length ?? 0);
</script>

<h1>概览</h1>

<div class="grid">
  <div class="card stat">
    <div class="stat-label">隧道总数</div>
    <div class="stat-value">{totalTunnels}</div>
  </div>
  <div class="card stat">
    <div class="stat-label">运行中</div>
    <div class="stat-value" style="color: #a6e3a1">{runningCount}</div>
  </div>
  <div class="card stat">
    <div class="stat-label">登录状态</div>
    <div class="stat-value" style="color: {snapshot?.cert?.logged_in ? '#a6e3a1' : '#f9e2af'}; font-size: 18px;">
      {snapshot?.cert?.logged_in ? "已登录" : "未登录"}
    </div>
  </div>
  <div class="card stat">
    <div class="stat-label">cloudflared</div>
    <div class="stat-value" style="color: {snapshot?.settings.cloudflared_exe_path ? '#a6e3a1' : '#f9e2af'}; font-size: 14px;">
      {snapshot?.settings.cloudflared_exe_path ? "已就绪" : "未设置"}
    </div>
  </div>
</div>

<h2 style="margin-top: 24px">运行中的隧道</h2>
{#if !snapshot || runningCount === 0}
  <div class="card" style="text-align: center; color: #a6adc8;">
    暂无运行中的隧道。前往 <a href="/tunnels">隧道</a> 页管理。
  </div>
{:else}
  <div class="card" style="padding: 0;">
    <table>
      <thead>
        <tr>
          <th>名称</th>
          <th>域名</th>
          <th>状态</th>
        </tr>
      </thead>
      <tbody>
        {#each snapshot.tunnels as t}
          {#if isRunning(snapshot.running[t.id])}
            <tr>
              <td>{t.name}</td>
              <td><code>{t.hostname}</code></td>
              <td>
                <span class="dot" style="background: {statusColor(snapshot.running[t.id])}"></span>
                {statusText(snapshot.running[t.id])}
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 20px;
  }
  .stat {
    padding: 16px;
  }
  .stat-label {
    font-size: 12px;
    color: #a6adc8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }
  .stat-value {
    font-size: 28px;
    font-weight: 700;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-right: 6px;
    vertical-align: middle;
  }
</style>
