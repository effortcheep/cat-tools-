import { createSignal, Show } from "solid-js";
import styles from "./FileDropZone.module.css";

interface FileDropZoneProps {
  onFilesSelected: (files: File[]) => void;
}

export function FileDropZone(props: FileDropZoneProps) {
  const [isDragging, setIsDragging] = createSignal(false);
  const fileInputRef: { current?: HTMLInputElement } = {};

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      const pdfFiles = Array.from(files).filter(
        (file) => file.type === "application/pdf" || file.name.toLowerCase().endsWith(".pdf")
      );
      if (pdfFiles.length > 0) {
        props.onFilesSelected(pdfFiles);
      }
    }
  };

  const handleClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileInput = (e: Event) => {
    const target = e.target as HTMLInputElement;
    const files = target.files;
    if (files && files.length > 0) {
      const pdfFiles = Array.from(files).filter(
        (file) => file.type === "application/pdf" || file.name.toLowerCase().endsWith(".pdf")
      );
      if (pdfFiles.length > 0) {
        props.onFilesSelected(pdfFiles);
      }
    }
    // Reset input
    target.value = "";
  };

  return (
    <div
      class={styles.dropZone}
      classList={{ [styles.dragging]: isDragging() }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onClick={handleClick}
    >
      <input
        ref={(el) => (fileInputRef.current = el)}
        type="file"
        accept=".pdf"
        multiple
        style={{ display: "none" }}
        onChange={handleFileInput}
      />
      <div class={styles.content}>
        <div class={styles.icon}>📄</div>
        <Show
          when={!isDragging()}
          fallback={<p class={styles.text}>释放文件以上传</p>}
        >
          <p class={styles.text}>
            <strong>点击选择文件</strong> 或拖拽 PDF 文件到此处
          </p>
          <p class={styles.hint}>支持多个 PDF 文件</p>
        </Show>
      </div>
    </div>
  );
}
