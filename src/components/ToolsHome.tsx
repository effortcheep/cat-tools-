import { createSignal, Show, For } from "solid-js";
import { PdfPrinterPage } from "./PdfPrinterPage";
import { CronGeneratorPage } from "./CronGenerator";
import { PortCheckerPage } from "./PortChecker";
import styles from "./ToolsHome.module.css";

interface Tool {
  id: string;
  name: string;
  description: string;
  icon: string;
  component: () => JSX.Element;
}

interface Category {
  id: string;
  name: string;
  icon: string;
  tools: Tool[];
}

export function ToolsHome() {
  const [activeTool, setActiveTool] = createSignal<string | null>(null);

  const categories: Category[] = [
    {
      id: "file",
      name: "文件处理",
      icon: "📁",
      tools: [
        {
          id: "pdf-printer",
          name: "批量打印 PDF",
          description: "选择多个 PDF 文件，批量发送到打印机",
          icon: "📄",
          component: PdfPrinterPage,
        },
      ],
    },
    {
      id: "dev",
      name: "开发工具",
      icon: "💻",
      tools: [
        {
          id: "cron-generator",
          name: "Cron 表达式生成器",
          description: "可视化生成 Cron 表达式，支持秒级精度",
          icon: "⏰",
          component: CronGeneratorPage,
        },
      ],
    },
    {
      id: "system",
      name: "系统工具",
      icon: "⚙️",
      tools: [
        {
          id: "port-checker",
          name: "端口占用查看",
          description: "查看系统端口占用情况，结束占用进程",
          icon: "🔌",
          component: PortCheckerPage,
        },
      ],
    },
  ];

  const allTools = categories.flatMap((c) => c.tools);

  const handleToolClick = (toolId: string) => {
    setActiveTool(toolId);
  };

  const handleBack = () => {
    setActiveTool(null);
  };

  return (
    <div class={styles.container}>
      <Show
        when={!activeTool()}
        fallback={
          <div class={styles.toolContainer}>
            <div class={styles.header}>
              <button class={styles.backButton} onClick={handleBack}>
                ← 返回工具列表
              </button>
            </div>
            {allTools.find((t) => t.id === activeTool())?.component()}
          </div>
        }
      >
        <h1 class={styles.title}>🛠️ Cat Tools</h1>
        <p class={styles.subtitle}>选择一个工具开始使用</p>

        <div class={styles.categories}>
          <For each={categories}>
            {(category) => (
              <div class={styles.category}>
                <div class={styles.categoryHeader}>
                  <span class={styles.categoryIcon}>{category.icon}</span>
                  <h2 class={styles.categoryName}>{category.name}</h2>
                </div>
                <div class={styles.toolsRow}>
                  <For each={category.tools}>
                    {(tool) => (
                      <div
                        class={styles.toolCard}
                        onClick={() => handleToolClick(tool.id)}
                      >
                        <div class={styles.toolIcon}>{tool.icon}</div>
                        <h3 class={styles.toolName}>{tool.name}</h3>
                        <p class={styles.toolDescription}>{tool.description}</p>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            )}
          </For>
        </div>

        <div class={styles.footer}>
          <p>更多工具开发中...</p>
        </div>
      </Show>
    </div>
  );
}

import type { JSX } from "solid-js";
