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
            .and_then(|v| v.as_bool())
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
) -> Result<(), String> {
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
) -> Result<(), String> {
    // Method: Use Microsoft Print to PDF or system printer via PowerShell
    let copies = copies.unwrap_or(1);
    
    // Convert to absolute path
    let abs_path = std::fs::canonicalize(pdf_path)
        .map_err(|e| format!("Failed to resolve path: {}", e))?
        .to_string_lossy()
        .to_string();
    
    // PowerShell script to print PDF silently
    let ps_script = format!(
        r#"
        $printer = "{}"
        $file = "{}"
        $copies = {}
        
        # Create a print document
        Add-Type -AssemblyName System.Drawing
        
        # Use Start-Process to open PDF with default handler and print
        $startInfo = New-Object System.Diagnostics.ProcessStartInfo
        $startInfo.FileName = " rundll32.exe"
        $startInfo.Arguments = "shell32.dll,ShellExec_RunDLL `"$file`"", ",Print"
        $startInfo.Verb = "Print"
        $startInfo.UseShellExecute = $true
        
        $process = [System.Diagnostics.Process]::Start($startInfo)
        if ($process) {{
            Start-Sleep -Milliseconds 500
        }}
        "#,
        printer_name.replace('"', "\\\""),
        abs_path.replace('"', "\\\""),
        copies
    );
    
    let output = Command::new("powershell")
        .args(["-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute print command: {}", e))?;
    
    if !output.status.success() {
        let _stderr = String::from_utf8_lossy(&output.stderr);
        // Even if there's stderr, the print might have succeeded
        // Try alternative method using Adobe Reader or Edge
        return print_pdf_windows_alternative(printer_name, pdf_path, copies).await;
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
async fn print_pdf_windows_alternative(
    printer_name: &str, 
    pdf_path: &str,
    copies: u32,
) -> Result<(), String> {
    // Try using Adobe Acrobat Reader if installed
    let reader_paths = [
        r"C:\Program Files\Adobe\Acrobat DC\Acrobat\Acrobat.exe",
        r"C:\Program Files (x86)\Adobe\Acrobat Reader DC\Reader\AcroRd32.exe",
        r"C:\Program Files\Adobe\Acrobat Reader DC\Reader\AcroRd32.exe",
    ];
    
    for reader_path in &reader_paths {
        if std::path::Path::new(reader_path).exists() {
            let args = if copies > 1 {
                format!(
                    "/t \"{}\" \"{}\" /n",
                    pdf_path,
                    printer_name
                )
            } else {
                format!(
                    "/t \"{}\" \"{}\"",
                    pdf_path,
                    printer_name
                )
            };
            
            let _ = Command::new(reader_path)
                .args(args.split_whitespace())
                .spawn()
                .map_err(|e| format!("Failed to launch Adobe Reader: {}", e))?;
            
            return Ok(());
        }
    }
    
    // Fallback: Use Microsoft Edge
    let edge_paths = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];
    
    for edge_path in &edge_paths {
        if std::path::Path::new(edge_path).exists() {
            let _ = Command::new(edge_path)
                .args(["--headless", "--print-to-pdf-no-header", pdf_path])
                .spawn()
                .map_err(|e| format!("Failed to launch Edge: {}", e))?;
            
            return Ok(());
        }
    }
    
    // Last resort: Use system default print
    let _ = Command::new("cmd")
        .args(["/c", "start", "/min", "", "print", pdf_path])
        .spawn()
        .map_err(|e| format!("Failed to execute print command: {}", e))?;
    
    Ok(())
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
) -> Result<(), String> {
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
    
    Ok(())
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
) -> Result<(), String> {
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
    
    Ok(())
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
