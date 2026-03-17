import { For, Show } from "solid-js";
import styles from "./FileList.module.css";

interface FileItem {
  id: string;
  file: File;
  path?: string;
  status: "pending" | "printing" | "completed" | "error";
  errorMessage?: string;
}

interface FileListProps {
  files: FileItem[];
  onRemove: (id: string) => void;
  onClear: () => void;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

function getStatusIcon(status: FileItem["status"]) {
  switch (status) {
    case "pending":
      return "⏳";
    case "printing":
      return "🖨️";
    case "completed":
      return "✅";
    case "error":
      return "❌";
    default:
      return "📄";
  }
}

export function FileList(props: FileListProps) {
  return (
    <Show when={props.files.length > 0}>
      <div class={styles.container}>
        <div class={styles.header}>
          <span class={styles.title}>
            已选择 {props.files.length} 个文件
          </span>
          <button class={styles.clearButton} onClick={props.onClear}>
            清空列表
          </button>
        </div>

        <div class={styles.list}>
          <For each={props.files}>
            {(item) => (
              <div class={styles.fileItem}>
                <div class={styles.fileIcon}>📄</div>
                <div class={styles.fileInfo}>
                  <div class={styles.fileName}>{item.file.name}</div>
                  <div class={styles.fileMeta}>
                    {formatFileSize(item.file.size)}
                    <Show when={item.status !== "pending"}>
                      <span class={styles.status}>
                        {getStatusIcon(item.status)}
                        {item.status === "printing" && " 打印中..."}
                        {item.status === "completed" && " 完成"}
                        {item.status === "error" && ` 失败: ${item.errorMessage}`}
                      </span>
                    </Show>
                  </div>
                </div>
                <Show when={item.status === "pending"}>
                  <button
                    class={styles.removeButton}
                    onClick={() => props.onRemove(item.id)}
                    title="移除文件"
                  >
                    ❌
                  </button>
                </Show>
              </div>
            )}
          </For>
        </div>
      </div>
    </Show>
  );
}
