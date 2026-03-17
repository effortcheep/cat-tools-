import { createSignal, onMount, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import styles from "./PrinterSelector.module.css";

interface Printer {
  name: string;
  is_default: boolean;
  status: string;
}

interface PrinterSelectorProps {
  selectedPrinter: string;
  onSelect: (printer: string) => void;
}

export function PrinterSelector(props: PrinterSelectorProps) {
  const [printers, setPrinters] = createSignal<Printer[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const loadPrinters = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Printer[]>("get_printers");
      setPrinters(result);
      
      // Select default printer if none selected
      if (!props.selectedPrinter && result.length > 0) {
        const defaultPrinter = result.find((p) => p.is_default);
        if (defaultPrinter) {
          props.onSelect(defaultPrinter.name);
        } else {
          props.onSelect(result[0].name);
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    loadPrinters();
  });

  return (
    <div class={styles.container}>
      <label class={styles.label}>🖨️ 选择打印机</label>
      
      <Show when={loading()}>
        <div class={styles.loading}>加载打印机列表...</div>
      </Show>

      <Show when={error()}>
        <div class={styles.error}>
          加载失败: {error()}
          <button class={styles.retryButton} onClick={loadPrinters}>
            重试
          </button>
        </div>
      </Show>

      <Show when={!loading() && !error()}>
        <Show
          when={printers().length > 0}
          fallback={<div class={styles.empty}>未找到打印机</div>}
        >
          <select
            class={styles.select}
            value={props.selectedPrinter}
            onChange={(e) => props.onSelect(e.currentTarget.value)}
          >
            <For each={printers()}>
              {(printer) => (
                <option value={printer.name}>
                  {printer.name} {printer.is_default ? "(默认)" : ""}
                </option>
              )}
            </For>
          </select>
          <div class={styles.hint}>
            共 {printers().length} 台打印机可用
          </div>
        </Show>
      </Show>
    </div>
  );
}
