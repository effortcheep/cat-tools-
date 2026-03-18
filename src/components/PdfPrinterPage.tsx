import { createSignal, createUniqueId, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { FileDropZone, FileList, PrinterSelector } from "./PdfPrinter";
import styles from "./PdfPrinterPage.module.css";

interface FileItem {
  id: string;
  file: File;
  path?: string;
  status: "pending" | "printing" | "completed" | "error";
  errorMessage?: string;
}

export function PdfPrinterPage() {
  const [files, setFiles] = createSignal<FileItem[]>([]);
  const [selectedPrinter, setSelectedPrinter] = createSignal("");
  const [isPrinting, setIsPrinting] = createSignal(false);
  const [overallProgress, setOverallProgress] = createSignal("");

  const handleFilesSelected = (newFiles: File[]) => {
    const newItems: FileItem[] = newFiles.map((file) => ({
      id: createUniqueId(),
      file,
      status: "pending",
    }));
    setFiles((prev) => [...prev, ...newItems]);
  };

  const handleRemoveFile = (id: string) => {
    setFiles((prev) => prev.filter((f) => f.id !== id));
  };

  const handleClearFiles = () => {
    setFiles([]);
    setOverallProgress("");
  };

  const handlePrint = async () => {
    const printer = selectedPrinter();
    if (!printer || files().length === 0) return;

    setIsPrinting(true);
    const totalFiles = files().length;
    let completedCount = 0;

    // Update all files to pending
    setFiles((prev) =>
      prev.map((f) => ({ ...f, status: "pending" as const, errorMessage: undefined }))
    );

    for (let i = 0; i < files().length; i++) {
      const fileItem = files()[i];
      
      // Update status to printing
      setFiles((prev) =>
        prev.map((f) =>
          f.id === fileItem.id ? { ...f, status: "printing" as const } : f
        )
      );

      setOverallProgress(`正在打印 ${i + 1}/${totalFiles}: ${fileItem.file.name}`);

      try {
        // Get file path from Tauri
        // Note: We need to use the opener plugin or file dialog to get actual paths
        // For now, we'll try to use the file object directly
        
        // Convert File to ArrayBuffer and save temporarily
        const arrayBuffer = await fileItem.file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);
        
        // Save to temp file using Tauri API
        const tempPath = await invoke<string>("save_temp_pdf", {
          filename: fileItem.file.name,
          data: Array.from(uint8Array),
        });

        // Print the PDF
        const printMethod = await invoke<string>("print_pdf", {
          printerName: printer,
          pdfPath: tempPath,
          copies: 1,
        });

        // Update status to completed with print method info
        setFiles((prev) =>
          prev.map((f) =>
            f.id === fileItem.id ? { 
              ...f, 
              status: "completed" as const,
              errorMessage: printMethod // Store print method info
            } : f
          )
        );
        completedCount++;

        // Clean up temp file
        await invoke("delete_temp_file", { path: tempPath });
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        setFiles((prev) =>
          prev.map((f) =>
            f.id === fileItem.id
              ? { ...f, status: "error" as const, errorMessage: errorMsg }
              : f
          )
        );
      }
    }

    setIsPrinting(false);
    setOverallProgress(`打印完成: ${completedCount}/${totalFiles} 个文件成功`);
  };

  const pendingFilesCount = () =>
    files().filter((f) => f.status === "pending").length;

  return (
    <div class={styles.container}>
      <FileDropZone onFilesSelected={handleFilesSelected} />

      <FileList
        files={files()}
        onRemove={handleRemoveFile}
        onClear={handleClearFiles}
      />

      <PrinterSelector
        selectedPrinter={selectedPrinter()}
        onSelect={setSelectedPrinter}
      />

      <Show when={files().length > 0}>
        <div class={styles.actions}>
          <button
            class={styles.printButton}
            disabled={isPrinting() || pendingFilesCount() === 0}
            onClick={handlePrint}
          >
            {isPrinting() ? "🖨️ 打印中..." : `🖨️ 开始打印 (${pendingFilesCount()}个文件)`}
          </button>
        </div>

        <Show when={overallProgress()}>
          <div class={styles.progress}>{overallProgress()}</div>
        </Show>
      </Show>
    </div>
  );
}
