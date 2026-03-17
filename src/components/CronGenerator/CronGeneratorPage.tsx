import { createSignal, createEffect, Show, For } from "solid-js";
import cronstrue from "cronstrue";
import { format } from "date-fns";
import { zhCN } from "date-fns/locale";
import { Cron } from "croner";
import styles from "./CronGeneratorPage.module.css";

const PRESETS = [
  { name: "每秒", cron: "* * * * * *", desc: "每分钟每秒执行" },
  { name: "每分钟", cron: "0 * * * * *", desc: "每分钟的第0秒执行" },
  { name: "每小时", cron: "0 0 * * * *", desc: "每小时的第0分0秒执行" },
  { name: "每天", cron: "0 0 0 * * *", desc: "每天0点0分0秒执行" },
  { name: "工作日", cron: "0 0 0 * * 1-5", desc: "周一至周五0点执行" },
  { name: "周末", cron: "0 0 0 * * 0,6", desc: "周六日0点执行" },
  { name: "每周", cron: "0 0 0 * * 0", desc: "每周日0点执行" },
  { name: "每月1号", cron: "0 0 0 1 * *", desc: "每月1号0点执行" },
];

const WEEK_DAYS = ["日", "一", "二", "三", "四", "五", "六"];

export function CronGeneratorPage() {
  const [seconds, setSeconds] = createSignal("0");
  const [minutes, setMinutes] = createSignal("0");
  const [hours, setHours] = createSignal("0");
  const [dayOfMonth, setDayOfMonth] = createSignal("*");
  const [month, setMonth] = createSignal("*");
  const [dayOfWeek, setDayOfWeek] = createSignal("?");
  const [description, setDescription] = createSignal("");
  const [nextExecutions, setNextExecutions] = createSignal<string[]>([]);
  const [error, setError] = createSignal("");
  const [copied, setCopied] = createSignal(false);

  const cronExpression = () => {
    return `${seconds()} ${minutes()} ${hours()} ${dayOfMonth()} ${month()} ${dayOfWeek()}`;
  };

  createEffect(() => {
    try {
      const expr = cronExpression();
      const desc = cronstrue.toString(expr, { locale: "zh_CN" });
      setDescription(desc);
      setError("");

      // Calculate next 5 executions
      const next = calculateNextExecutions(expr, 5);
      setNextExecutions(next);
    } catch (err) {
      setDescription("");
      setError("无效的 Cron 表达式");
      setNextExecutions([]);
    }
  });

  const calculateNextExecutions = (cronExpr: string, count: number): string[] => {
    try {
      const cron = new Cron(cronExpr);
      // nextRuns(n) returns an array of the next n execution dates
      const nextDates = cron.nextRuns(count);
      
      if (!nextDates || nextDates.length === 0) {
        return [];
      }
      
      return nextDates.map(date => format(date, "yyyy-MM-dd HH:mm:ss"));
    } catch (err) {
      console.error("Cron calculation error:", err, "Expression:", cronExpr);
      return [];
    }
  };

  const handlePresetClick = (preset: typeof PRESETS[0]) => {
    const parts = preset.cron.split(" ");
    setSeconds(parts[0]);
    setMinutes(parts[1]);
    setHours(parts[2]);
    setDayOfMonth(parts[3]);
    setMonth(parts[4]);
    setDayOfWeek(parts[5]);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(cronExpression());
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div class={styles.container}>
      {/* Presets */}
      <div class={styles.section}>
        <h3 class={styles.sectionTitle}>快捷预设</h3>
        <div class={styles.presets}>
          <For each={PRESETS}>
            {(preset) => (
              <button
                class={styles.presetButton}
                onClick={() => handlePresetClick(preset)}
                title={preset.desc}
              >
                {preset.name}
              </button>
            )}
          </For>
        </div>
      </div>

      {/* Cron Fields */}
      <div class={styles.section}>
        <h3 class={styles.sectionTitle}>自定义设置</h3>
        <div class={styles.fields}>
          <div class={styles.field}>
            <label class={styles.fieldLabel}>秒</label>
            <select
              class={styles.fieldSelect}
              value={seconds()}
              onChange={(e) => setSeconds(e.currentTarget.value)}
            >
              <option value="*">每秒 (*)</option>
              <For each={Array.from({ length: 60 }, (_, i) => i)}>
                {(i) => <option value={i.toString()}>{i}秒</option>}
              </For>
              <option value="*/5">每5秒</option>
              <option value="*/10">每10秒</option>
              <option value="*/15">每15秒</option>
              <option value="*/30">每30秒</option>
            </select>
          </div>

          <div class={styles.field}>
            <label class={styles.fieldLabel}>分</label>
            <select
              class={styles.fieldSelect}
              value={minutes()}
              onChange={(e) => setMinutes(e.currentTarget.value)}
            >
              <option value="*">每分 (*)</option>
              <For each={Array.from({ length: 60 }, (_, i) => i)}>
                {(i) => <option value={i.toString()}>{i}分</option>}
              </For>
              <option value="*/5">每5分</option>
              <option value="*/10">每10分</option>
              <option value="*/15">每15分</option>
              <option value="*/30">每30分</option>
            </select>
          </div>

          <div class={styles.field}>
            <label class={styles.fieldLabel}>时</label>
            <select
              class={styles.fieldSelect}
              value={hours()}
              onChange={(e) => setHours(e.currentTarget.value)}
            >
              <option value="*">每小时 (*)</option>
              <For each={Array.from({ length: 24 }, (_, i) => i)}>
                {(i) => <option value={i.toString()}>{i}点</option>}
              </For>
            </select>
          </div>

          <div class={styles.field}>
            <label class={styles.fieldLabel}>日</label>
            <select
              class={styles.fieldSelect}
              value={dayOfMonth()}
              onChange={(e) => {
                setDayOfMonth(e.currentTarget.value);
                if (e.currentTarget.value !== "?") {
                  setDayOfWeek("?");
                }
              }}
            >
              <option value="*">每日 (*)</option>
              <option value="?">不指定 (?)</option>
              <For each={Array.from({ length: 31 }, (_, i) => i + 1)}>
                {(i) => <option value={i.toString()}>{i}日</option>}
              </For>
            </select>
          </div>

          <div class={styles.field}>
            <label class={styles.fieldLabel}>月</label>
            <select
              class={styles.fieldSelect}
              value={month()}
              onChange={(e) => setMonth(e.currentTarget.value)}
            >
              <option value="*">每月 (*)</option>
              <For each={Array.from({ length: 12 }, (_, i) => i + 1)}>
                {(i) => <option value={i.toString()}>{i}月</option>}
              </For>
            </select>
          </div>

          <div class={styles.field}>
            <label class={styles.fieldLabel}>周</label>
            <select
              class={styles.fieldSelect}
              value={dayOfWeek()}
              onChange={(e) => {
                setDayOfWeek(e.currentTarget.value);
                if (e.currentTarget.value !== "?") {
                  setDayOfMonth("?");
                }
              }}
            >
              <option value="*">每日 (*)</option>
              <option value="?">不指定 (?)</option>
              <For each={WEEK_DAYS.map((d, i) => ({ day: d, value: i }))}>
                {(item) => (
                  <option value={item.value.toString()}>周{item.day}</option>
                )}
              </For>
              <option value="1-5">周一至周五</option>
              <option value="0,6">周末</option>
            </select>
          </div>
        </div>
      </div>

      {/* Result */}
      <div class={styles.section}>
        <h3 class={styles.sectionTitle}>生成结果</h3>
        <div class={styles.result}>
          <div class={styles.expression}>
            <code>{cronExpression()}</code>
            <button
              class={styles.copyButton}
              onClick={handleCopy}
              classList={{ [styles.copied]: copied() }}
            >
              {copied() ? "✓ 已复制" : "📋 复制"}
            </button>
          </div>

          <Show when={error()}>
            <div class={styles.error}>{error()}</div>
          </Show>

          <Show when={!error() && description()}>
            <div class={styles.description}>
              <strong>描述：</strong>
              {description()}
            </div>
          </Show>
        </div>
      </div>

      {/* Next Executions */}
      <Show when={!error() && nextExecutions().length > 0}>
        <div class={styles.section}>
          <h3 class={styles.sectionTitle}>预计执行时间（示例）</h3>
          <ul class={styles.executionList}>
            <For each={nextExecutions()}>
              {(time, index) => (
                <li class={styles.executionItem}>
                  <span class={styles.executionNumber}>{index() + 1}</span>
                  {time}
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
}
