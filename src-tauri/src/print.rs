use std::process::Command;
use std::fs;
use std::path::PathBuf;
use serde::Serialize;

#[cfg(target_os = "windows")]
use encoding_rs::GBK;

/// Save temporary PDF file from bytes
#[tauri::command]
pub fn save_temp_pdf(filename: String, data: Vec<u8>) -> Result<String, String> {
    // Create temp directory if not exists
    let temp_dir = std::env::temp_dir().join("cat-tools-print");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    // Create a unique filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    let safe_filename = filename.replace(|c: char| !c.is_alphanumeric() && c != '.', "_");
    let temp_path = temp_dir.join(format!("{}_{}", timestamp, safe_filename));
    
    // Write the file
    fs::write(&temp_path, data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    Ok(temp_path.to_string_lossy().to_string())
}

/// Delete temporary file
#[tauri::command]
pub fn delete_temp_file(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete temp file: {}", e))?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
pub struct Printer {
    pub name: String,
    pub is_default: bool,
    pub status: String,
}

impl Printer {
    fn from_powershell_json(json: &serde_json::Value) -> Option<Self> {
        let name = json.get("Name")?.as_str()?.to_string();
        let is_default = json.get("IsDefault")
            .and_then(|v| {
                if v.is_null() {
                    Some(false)
                } else {
                    v.as_bool()
                }
            })
            .unwrap_or(false);
        
        Some(Printer {
            name,
            is_default,
            status: "Ready".to_string(),
        })
    }
}

/// Get list of available printers (cross-platform)
#[tauri::command]
pub async fn get_printers() -> Result<Vec<Printer>, String> {
    #[cfg(target_os = "windows")]
    {
        get_windows_printers().await
    }
    
    #[cfg(target_os = "macos")]
    {
        get_macos_printers().await
    }
    
    #[cfg(target_os = "linux")]
    {
        get_linux_printers().await
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

/// Print PDF file silently
#[tauri::command]
pub async fn print_pdf(
    printer_name: String, 
    pdf_path: String,
    copies: Option<u32>,
) -> Result<String, String> {
    // Verify file exists
    if !std::path::Path::new(&pdf_path).exists() {
        return Err(format!("File not found: {}", pdf_path));
    }
    
    // Verify it's a PDF
    if !pdf_path.to_lowercase().ends_with(".pdf") {
        return Err("File must be a PDF".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        print_pdf_windows(&printer_name, &pdf_path, copies).await
    }
    
    #[cfg(target_os = "macos")]
    {
        print_pdf_macos(&printer_name, &pdf_path, copies).await
    }
    
    #[cfg(target_os = "linux")]
    {
        print_pdf_linux(&printer_name, &pdf_path, copies).await
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

// ==================== Windows Implementation ====================

#[cfg(target_os = "windows")]
async fn get_windows_printers() -> Result<Vec<Printer>, String> {
    // Use PowerShell with UTF-8 encoding to avoid encoding issues
    get_windows_printers_powershell().await
}

#[cfg(target_os = "windows")]
async fn get_windows_printers_powershell() -> Result<Vec<Printer>, String> {
    // Use PowerShell with UTF-8 encoding
    let ps_script = r#"
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
        $OutputEncoding = [System.Text.Encoding]::UTF8
        Get-Printer | Select-Object Name, IsDefault | ConvertTo-Json -Compress
    "#;
    
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command", ps_script
        ])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;
    
    // Try UTF-8 first, fallback to GBK if needed
    let stdout = if let Ok(text) = String::from_utf8(output.stdout.clone()) {
        text
    } else {
        // Fallback to GBK encoding for Chinese Windows
        let (text, _, _) = GBK.decode(&output.stdout);
        text.into_owned()
    };
    
    // Parse JSON response
    if stdout.trim().is_empty() {
        return Ok(Vec::new());
    }
    
    // Parse as raw JSON first
    let json_value: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    // Convert to printers using our custom parser
    let printers: Vec<Printer> = if let Some(array) = json_value.as_array() {
        array.iter()
            .filter_map(Printer::from_powershell_json)
            .collect()
    } else {
        // Single object
        Printer::from_powershell_json(&json_value)
            .map(|p| vec![p])
            .unwrap_or_default()
    };
    
    Ok(printers)
}

#[cfg(target_os = "windows")]
async fn print_pdf_windows(
    printer_name: &str, 
    pdf_path: &str,
    copies: Option<u32>,
) -> Result<String, String> {
    let copies = copies.unwrap_or(1);
    
    // Convert to absolute path
    let abs_path = std::fs::canonicalize(pdf_path)
        .map_err(|e| format!("步骤0-路径解析失败: 无法解析PDF路径 '{}': {}", pdf_path, e))?
        .to_string_lossy()
        .to_string();
    
    // Remove UNC prefix (\\?\) if present - many external programs can't handle UNC paths
    let abs_path = if abs_path.starts_with(r"\\?\") {
        abs_path[4..].to_string()
    } else {
        abs_path
    };
    
    // Use SumatraPDF only
    try_sumatrapdf(printer_name, &abs_path, copies).await.map_err(|e| {
        format!("{}", e)
    })
}

#[cfg(target_os = "windows")]
async fn try_sumatrapdf(printer_name: &str, pdf_path: &str, _copies: u32) -> Result<String, String> {
    let mut sumatra_paths = vec![];
    
    // 1. 首先检查与可执行文件同目录（打包后的标准位置）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let sumatra_in_exe_dir = exe_dir.join("SumatraPDF.exe");
            sumatra_paths.push(sumatra_in_exe_dir.to_string_lossy().to_string());
            
            // 2. 检查 resources 目录（Tauri 打包后的资源目录）
            let sumatra_in_resources = exe_dir.join("resources").join("SumatraPDF.exe");
            sumatra_paths.push(sumatra_in_resources.to_string_lossy().to_string());
        }
    }
    
    // 3. 开发模式：检查项目根目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // 开发时：target/debug/ -> 上溯查找项目根目录
            let project_root = exe_dir
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent());
            if let Some(root) = project_root {
                let sumatra_in_root = root.join("SumatraPDF.exe");
                sumatra_paths.push(sumatra_in_root.to_string_lossy().to_string());
            }
        }
    }
    
    // 4. 系统安装路径（作为备选）
    sumatra_paths.push(r"C:\Program Files\SumatraPDF\SumatraPDF.exe".to_string());
    sumatra_paths.push(r"C:\Program Files (x86)\SumatraPDF\SumatraPDF.exe".to_string());
    
    let mut checked_paths = Vec::new();
    
    for sumatra_path in &sumatra_paths {
        checked_paths.push(sumatra_path.to_string());
        
        if std::path::Path::new(&sumatra_path).exists() {
            if !std::path::Path::new(pdf_path).exists() {
                return Err(format!("PDF文件不存在: {}", pdf_path));
            }
            
            let args = vec![
                "-print-to".to_string(),
                printer_name.to_string(),
                pdf_path.to_string(),
            ];
            
            match Command::new(&sumatra_path).args(&args).output() {
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    if output.status.success() || stderr.is_empty() {
                        return Ok(format!("SumatraPDF ({})", sumatra_path));
                    } else {
                        return Err(format!("SumatraPDF打印失败: {}", stderr));
                    }
                }
                Err(e) => return Err(format!("启动SumatraPDF失败: {}", e)),
            }
        }
    }
    
    Err(format!(
        "未找到 SumatraPDF.exe\n\n已检查以下位置:\n{}\n\n请将 SumatraPDF.exe 放在以下任一位置:\n1. 与主程序 exe 同级目录\n2. 主程序目录下的 resources 文件夹\n3. 系统安装路径",
        checked_paths.join("\n")
    ))
}

// ==================== macOS Implementation ====================

#[cfg(target_os = "macos")]
async fn get_macos_printers() -> Result<Vec<Printer>, String> {
    let output = Command::new("lpstat")
        .args(["-p"])
        .output()
        .map_err(|e| format!("Failed to execute lpstat: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut printers = Vec::new();
    
    for line in stdout.lines() {
        // Parse: "printer HP_LaserJet is idle.  enabled since ..."
        if line.starts_with("printer ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                let status = if line.contains("is idle") {
                    "Ready"
                } else if line.contains("is printing") {
                    "Printing"
                } else {
                    "Unknown"
                }.to_string();
                
                printers.push(Printer {
                    name: name.clone(),
                    is_default: false, // Will be set below
                    status,
                });
            }
        }
    }
    
    // Get default printer
    if let Ok(output) = Command::new("lpstat").args(["-d"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(default_line) = stdout.lines().next() {
            if default_line.starts_with("system default destination: ") {
                let default_name = default_line.trim_start_matches("system default destination: ").trim();
                for printer in &mut printers {
                    if printer.name == default_name {
                        printer.is_default = true;
                    }
                }
            }
        }
    }
    
    Ok(printers)
}

#[cfg(target_os = "macos")]
async fn print_pdf_macos(
    printer_name: &str, 
    pdf_path: &str,
    copies: Option<u32>,
) -> Result<String, String> {
    let copies = copies.unwrap_or(1);
    
    // Convert to absolute path
    let abs_path = std::fs::canonicalize(pdf_path)
        .map_err(|e| format!("Failed to resolve path: {}", e))?
        .to_string_lossy()
        .to_string();
    
    let mut args = vec![
        "-d".to_string(),
        printer_name.to_string(),
        "-o".to_string(),
        "media=A4".to_string(),
        "-o".to_string(),
        "sides=one-sided".to_string(),
    ];
    
    if copies > 1 {
        args.push("-n".to_string());
        args.push(copies.to_string());
    }
    
    args.push(abs_path);
    
    let output = Command::new("lp")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute lp command: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Print failed: {}", stderr));
    }
    
    Ok("lp command".to_string())
}

// ==================== Linux Implementation ====================

#[cfg(target_os = "linux")]
async fn get_linux_printers() -> Result<Vec<Printer>, String> {
    let output = Command::new("lpstat")
        .args(["-p"])
        .output()
        .map_err(|e| format!("Failed to execute lpstat: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut printers = Vec::new();
    
    for line in stdout.lines() {
        // Parse: "printer HP_LaserJet is idle.  enabled since ..."
        if line.starts_with("printer ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                let status = if line.contains("is idle") {
                    "Ready"
                } else if line.contains("is printing") {
                    "Printing"
                } else {
                    "Unknown"
                }.to_string();
                
                printers.push(Printer {
                    name: name.clone(),
                    is_default: false,
                    status,
                });
            }
        }
    }
    
    // Get default printer
    if let Ok(output) = Command::new("lpstat").args(["-d"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(default_line) = stdout.lines().next() {
            if default_line.starts_with("system default destination: ") {
                let default_name = default_line.trim_start_matches("system default destination: ").trim();
                for printer in &mut printers {
                    if printer.name == default_name {
                        printer.is_default = true;
                    }
                }
            }
        }
    }
    
    Ok(printers)
}

#[cfg(target_os = "linux")]
async fn print_pdf_linux(
    printer_name: &str, 
    pdf_path: &str,
    copies: Option<u32>,
) -> Result<String, String> {
    let copies = copies.unwrap_or(1);
    
    // Convert to absolute path
    let abs_path = std::fs::canonicalize(pdf_path)
        .map_err(|e| format!("Failed to resolve path: {}", e))?
        .to_string_lossy()
        .to_string();
    
    let mut args = vec![
        "-d".to_string(),
        printer_name.to_string(),
        "-o".to_string(),
        "media=A4".to_string(),
        "-o".to_string(),
        "sides=one-sided".to_string(),
    ];
    
    if copies > 1 {
        args.push("-n".to_string());
        args.push(copies.to_string());
    }
    
    args.push(abs_path);
    
    let output = Command::new("lp")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute lp command: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Print failed: {}", stderr));
    }
    
    Ok("lp command".to_string())
}

/// Get default printer name
#[tauri::command]
pub async fn get_default_printer() -> Result<Option<String>, String> {
    let printers = get_printers().await?;
    
    // Find default printer
    if let Some(default) = printers.iter().find(|p| p.is_default) {
        return Ok(Some(default.name.clone()));
    }
    
    // Return first printer if no default
    if let Some(first) = printers.first() {
        return Ok(Some(first.name.clone()));
    }
    
    Ok(None)
}
