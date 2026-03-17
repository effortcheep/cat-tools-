use std::process::Command;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct PortInfo {
    pub protocol: String,
    pub local_address: String,
    pub local_port: u16,
    pub foreign_address: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
}

/// Get list of ports in use (cross-platform)
#[tauri::command]
pub async fn get_ports() -> Result<Vec<PortInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        get_windows_ports().await
    }
    
    #[cfg(target_os = "macos")]
    {
        get_macos_ports().await
    }
    
    #[cfg(target_os = "linux")]
    {
        get_linux_ports().await
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

/// Kill process by PID
#[tauri::command]
pub async fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        kill_process_windows(pid).await
    }
    
    #[cfg(target_os = "macos")]
    {
        kill_process_unix(pid).await
    }
    
    #[cfg(target_os = "linux")]
    {
        kill_process_unix(pid).await
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

// ==================== Windows Implementation ====================

#[cfg(target_os = "windows")]
async fn get_windows_ports() -> Result<Vec<PortInfo>, String> {
    // Get network connections
    let output = Command::new("netstat")
        .args(["-ano"])
        .output()
        .map_err(|e| format!("Failed to execute netstat: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();
    let mut pid_set = std::collections::HashSet::new();
    
    for line in stdout.lines() {
        // Parse: "  TCP    0.0.0.0:8080           0.0.0.0:0              LISTENING       1234"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 && (parts[0] == "TCP" || parts[0] == "UDP") {
            let protocol = parts[0].to_string();
            let local = parts[1];
            let foreign = parts[2];
            let state = if parts[0] == "UDP" {
                ""
            } else {
                parts[3]
            };
            let pid_str = if parts[0] == "UDP" {
                parts[3]
            } else {
                parts[4]
            };
            
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid > 0 {
                    pid_set.insert(pid);
                    
                    // Parse local address and port
                    let (local_addr, port) = parse_address_port(local);
                    
                    ports.push(PortInfo {
                        protocol,
                        local_address: local_addr,
                        local_port: port,
                        foreign_address: foreign.to_string(),
                        state: state.to_string(),
                        pid,
                        process_name: String::new(), // Will be filled later
                    });
                }
            }
        }
    }
    
    // Get process names
    let process_map = get_windows_process_names(&pid_set).await?;
    
    for port in &mut ports {
        if let Some(name) = process_map.get(&port.pid) {
            port.process_name = name.clone();
        }
    }
    
    // Sort by port number
    ports.sort_by(|a, b| a.local_port.cmp(&b.local_port));
    
    Ok(ports)
}

#[cfg(target_os = "windows")]
fn parse_address_port(addr: &str) -> (String, u16) {
    if let Some(pos) = addr.rfind(':') {
        let address = addr[..pos].to_string();
        if let Ok(port) = addr[pos + 1..].parse::<u16>() {
            return (address, port);
        }
    }
    (addr.to_string(), 0)
}

#[cfg(target_os = "windows")]
async fn get_windows_process_names(pids: &std::collections::HashSet<u32>) -> Result<std::collections::HashMap<u32, String>, String> {
    let mut process_map = std::collections::HashMap::new();
    
    // Get process list using tasklist
    let output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("Failed to execute tasklist: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        // Parse: "process.exe","1234","Console","1","12345 K"
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            let name = parts[0].trim_matches('"').to_string();
            if let Ok(pid) = parts[1].trim_matches('"').parse::<u32>() {
                if pids.contains(&pid) {
                    process_map.insert(pid, name);
                }
            }
        }
    }
    
    Ok(process_map)
}

#[cfg(target_os = "windows")]
async fn kill_process_windows(pid: u32) -> Result<(), String> {
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to execute taskkill: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("拒绝访问") || stderr.contains("access denied") {
            return Err("需要管理员权限才能结束此进程".to_string());
        }
        return Err(format!("结束进程失败: {}", stderr));
    }
    
    Ok(())
}

// ==================== macOS Implementation ====================

#[cfg(target_os = "macos")]
async fn get_macos_ports() -> Result<Vec<PortInfo>, String> {
    get_unix_ports().await
}

// ==================== Linux Implementation ====================

#[cfg(target_os = "linux")]
async fn get_linux_ports() -> Result<Vec<PortInfo>, String> {
    get_unix_ports().await
}

#[cfg(unix)]
async fn get_unix_ports() -> Result<Vec<PortInfo>, String> {
    // Get network connections using lsof
    let output = Command::new("lsof")
        .args(["-i", "-P", "-n", "-F", "pcnPt"])
        .output()
        .map_err(|e| format!("Failed to execute lsof: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();
    let mut current_pid = 0u32;
    let mut current_command = String::new();
    
    for line in stdout.lines() {
        if line.starts_with('p') {
            if let Ok(pid) = line[1..].parse::<u32>() {
                current_pid = pid;
            }
        } else if line.starts_with('c') {
            current_command = line[1..].to_string();
        } else if line.starts_with('n') {
            // Parse network info: "127.0.0.1:8080->127.0.0.1:12345"
            let addr = &line[1..];
            if let Some((local, rest)) = addr.split_once("->") {
                let (local_addr, port) = parse_address_port(local);
                let state = if rest.contains("ESTABLISHED") {
                    "ESTABLISHED"
                } else if rest.contains("LISTEN") {
                    "LISTENING"
                } else {
                    ""
                };
                
                ports.push(PortInfo {
                    protocol: "TCP".to_string(),
                    local_address: local_addr,
                    local_port: port,
                    foreign_address: rest.to_string(),
                    state: state.to_string(),
                    pid: current_pid,
                    process_name: current_command.clone(),
                });
            }
        }
    }
    
    Ok(ports)
}

#[cfg(unix)]
async fn kill_process_unix(pid: u32) -> Result<(), String> {
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to execute kill: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Operation not permitted") {
            return Err("需要管理员权限才能结束此进程".to_string());
        }
        return Err(format!("结束进程失败: {}", stderr));
    }
    
    Ok(())
}

#[cfg(not(unix))]
#[cfg(not(target_os = "windows"))]
async fn get_unix_ports() -> Result<Vec<PortInfo>, String> {
    Err("Not implemented".to_string())
}

#[cfg(not(unix))]
#[cfg(not(target_os = "windows"))]
async fn kill_process_unix(_pid: u32) -> Result<(), String> {
    Err("Not implemented".to_string())
}
