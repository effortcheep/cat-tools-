import { createSignal, createEffect, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import styles from "./PortCheckerPage.module.css";

interface PortInfo {
  protocol: string;
  local_address: string;
  local_port: number;
  foreign_address: string;
  state: string;
  pid: number;
  process_name: string;
}

export function PortCheckerPage() {
  const [ports, setPorts] = createSignal<PortInfo[]>([]);
  const [filteredPorts, setFilteredPorts] = createSignal<PortInfo[]>([]);
  const [searchQuery, setSearchQuery] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");
  const [killingPid, setKillingPid] = createSignal<number | null>(null);
  const [showPermissionDialog, setShowPermissionDialog] = createSignal(false);

  const loadPorts = async () => {
    setLoading(true);
    setError("");
    try {
      const result = await invoke<PortInfo[]>("get_ports");
      setPorts(result);
      setFilteredPorts(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    const query = searchQuery().toLowerCase();
    if (!query) {
      setFilteredPorts(ports());
    } else {
      const filtered = ports().filter(
        (port) =>
          port.local_port.toString().includes(query) ||
          port.process_name.toLowerCase().includes(query) ||
          port.local_address.toLowerCase().includes(query)
      );
      setFilteredPorts(filtered);
    }
  });

  const handleKill = async (port: PortInfo) => {
    if (!confirm(`确定要结束进程 "${port.process_name}" (PID: ${port.pid}) 吗？`)) {
      return;
    }

    setKillingPid(port.pid);
    try {
      await invoke("kill_process", { pid: port.pid });
      // Refresh the list after killing
      await loadPorts();
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      if (errMsg.includes("管理员权限") || errMsg.includes("permission")) {
        setShowPermissionDialog(true);
      } else {
        setError(errMsg);
      }
    } finally {
      setKillingPid(null);
    }
  };

  const getStateColor = (state: string) => {
    switch (state.toUpperCase()) {
      case "LISTENING":
        return styles.stateListening;
      case "ESTABLISHED":
        return styles.stateEstablished;
      case "TIME_WAIT":
        return styles.stateTimeWait;
      case "CLOSE_WAIT":
        return styles.stateCloseWait;
      default:
        return "";
    }
  };

  return (
    <div class={styles.container}>
      {/* Header */}
      <div class={styles.header}>
        <div class={styles.searchBox}>
          <input
            type="text"
            placeholder="搜索端口、进程名或地址..."
            value={searchQuery()}
            onInput={(e) => setSearchQuery(e.currentTarget.value)}
            class={styles.searchInput}
          />
        </div>
        <button
          class={styles.refreshButton}
          onClick={loadPorts}
          disabled={loading()}
        >
          {loading() ? "🔄 刷新中..." : "🔄 刷新列表"}
        </button>
      </div>

      {/* Error */}
      <Show when={error()}>
        <div class={styles.error}>{error()}</div>
      </Show>

      {/* Permission Dialog */}
      <Show when={showPermissionDialog()}>
        <div class={styles.dialogOverlay}>
          <div class={styles.dialog}>
            <h3>⚠️ 需要管理员权限</h3>
            <p>结束此进程需要管理员权限。请尝试以下方法：</p>
            <ul>
              <li>右键点击应用图标，选择"以管理员身份运行"</li>
              <li>或在命令行中使用管理员权限启动应用</li>
            </ul>
            <button
              class={styles.dialogButton}
              onClick={() => setShowPermissionDialog(false)}
            >
              知道了
            </button>
          </div>
        </div>
      </Show>

      {/* Port List */}
      <div class={styles.tableContainer}>
        <Show
          when={!loading()}
          fallback={<div class={styles.loading}>加载中...</div>}
        >
          <Show
            when={filteredPorts().length > 0}
            fallback={
              <div class={styles.empty}>
                {ports().length === 0
                  ? "点击刷新按钮查看端口列表"
                  : "没有找到匹配的端口"}
              </div>
            }
          >
            <table class={styles.table}>
              <thead>
                <tr>
                  <th>协议</th>
                  <th>本地地址</th>
                  <th>端口</th>
                  <th>状态</th>
                  <th>进程名</th>
                  <th>PID</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <For each={filteredPorts()}>
                  {(port) => (
                    <tr>
                      <td>
                        <span
                          class={styles.protocol}
                          classList={{
                            [styles.protocolTcp]:
                              port.protocol.toUpperCase() === "TCP",
                            [styles.protocolUdp]:
                              port.protocol.toUpperCase() === "UDP",
                          }}
                        >
                          {port.protocol}
                        </span>
                      </td>
                      <td>{port.local_address}</td>
                      <td>
                        <strong>{port.local_port}</strong>
                      </td>
                      <td>
                        <Show when={port.state}>
                          <span
                            class={`${styles.state} ${getStateColor(
                              port.state
                            )}`}
                          >
                            {port.state}
                          </span>
                        </Show>
                      </td>
                      <td>{port.process_name || "-"}</td>
                      <td>{port.pid || "-"}</td>
                      <td>
                        <Show when={port.pid > 0}>
                          <button
                            class={styles.killButton}
                            onClick={() => handleKill(port)}
                            disabled={killingPid() === port.pid}
                          >
                            {killingPid() === port.pid ? "结束中..." : "结束"}
                          </button>
                        </Show>
                      </td>
                    </tr>
                  )}
                </For>
              </tbody>
            </table>
          </Show>
        </Show>
      </div>

      {/* Footer */}
      <Show when={ports().length > 0}>
        <div class={styles.footer}>
          共 {filteredPorts().length} 个端口
          {filteredPorts().length !== ports().length &&
            ` (总计 ${ports().length} 个)`}
        </div>
      </Show>
    </div>
  );
}
