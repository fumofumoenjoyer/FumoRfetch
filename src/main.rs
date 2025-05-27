use std::process::Command;
use std::env;
use std::fs;
use std::time::Duration;
use std::path::Path;

// Struct to hold system information
struct SystemInfo {
    hostname: String,
    os: String,
    kernel: String,
    uptime: String,
    shell: String,
    terminal: Option<String>,
    packages: String,
    cpu: String,
    gpu: String,
    gpu_driver: String,
    memory: (String, String),  // Used / Total
}

fn main() {
    // Get system information
    let info = get_system_info();
    
    // Display the system information
    display_info(&info);
}

fn get_system_info() -> SystemInfo {
    SystemInfo {
        hostname: get_hostname(),
        os: get_os_info(),
        kernel: get_kernel_version(),
        uptime: get_uptime(),
        shell: get_shell(),
        terminal: get_terminal(),
        packages: get_package_count(),
        cpu: get_cpu_info(),
        gpu: get_gpu_info().0,
        gpu_driver: get_gpu_info().1,
        memory: get_memory_info(),
    }
}

fn get_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| String::from("Unknown"))
        .trim()
        .to_string()
}

fn get_os_info() -> String {
    if let Ok(os_release) = fs::read_to_string("/etc/os-release") {
        for line in os_release.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line.replacen("PRETTY_NAME=", "", 1)
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    String::from("Linux")
}

fn get_kernel_version() -> String {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .unwrap_or_else(|_| panic!("Failed to get kernel version"));
    
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn get_uptime() -> String {
    if let Ok(uptime_str) = fs::read_to_string("/proc/uptime") {
        if let Some(secs_str) = uptime_str.split_whitespace().next() {
            if let Ok(secs) = secs_str.parse::<f64>() {
                return format_uptime(Duration::from_secs_f64(secs));
            }
        }
    }
    String::from("Unknown")
}

fn format_uptime(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn get_shell() -> String {
    env::var("SHELL")
        .unwrap_or_else(|_| String::from("Unknown"))
        .split('/')
        .last()
        .unwrap_or("Unknown")
        .to_string()
}

fn get_terminal() -> Option<String> {
    env::var("TERM").ok()
}

fn get_package_count() -> String {
    // Try to detect package manager and count packages
    // This is a simplified version
    
    // Check for apt (Debian/Ubuntu)
    if let Ok(output) = Command::new("dpkg").args(&["--get-selections"]).output() {
        let count = String::from_utf8_lossy(&output.stdout).lines().count();
        return format!("{} (apt)", count);
    }
    
    // Check for pacman (Arch)
    if let Ok(output) = Command::new("pacman").args(&["-Q"]).output() {
        let count = String::from_utf8_lossy(&output.stdout).lines().count();
        return format!("{} (pacman)", count);
    }
    
    // Check for dnf/yum (Fedora/RHEL)
    if let Ok(output) = Command::new("rpm").args(&["-qa"]).output() {
        let count = String::from_utf8_lossy(&output.stdout).lines().count();
        return format!("{} (rpm)", count);
    }
    
    String::from("Unknown")
}

fn get_cpu_info() -> String {
    if let Ok(cpu_info) = fs::read_to_string("/proc/cpuinfo") {
        for line in cpu_info.lines() {
            if line.starts_with("model name") {
                return line.split(':')
                    .nth(1)
                    .unwrap_or("Unknown")
                    .trim()
                    .to_string();
            }
        }
    }
    String::from("Unknown CPU")
}

fn get_memory_info() -> (String, String) {
    let mut total = 0;
    let mut available = 0;
    
    if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    if let Ok(kbytes) = value.parse::<u64>() {
                        total = kbytes;
                    }
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    if let Ok(kbytes) = value.parse::<u64>() {
                        available = kbytes;
                    }
                }
            }
        }
    }
    
    // Convert to human-readable format (MB or GB)
    let used = total - available;
    let used_str = format_memory_size(used);
    let total_str = format_memory_size(total);
    
    (used_str, total_str)
}

fn format_memory_size(size_kb: u64) -> String {
    let size_mb = size_kb as f64 / 1024.0;
    
    if size_mb > 1024.0 {
        let size_gb = size_mb / 1024.0;
        format!("{:.2} GB", size_gb)
    } else {
        format!("{:.2} MB", size_mb)
    }
}

fn get_gpu_info() -> (String, String) {
    // Try multiple methods to detect GPU
    
    // Try lspci first (most universal)
    if let Ok(output) = Command::new("lspci").output() {
        let lspci_output = String::from_utf8_lossy(&output.stdout);
        
        // Look for graphics cards in lspci output
        for line in lspci_output.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("vga") || 
               line_lower.contains("display") || 
               line_lower.contains("3d") ||
               line_lower.contains("graphics") {
                
                // Extract the GPU model from the line
                if let Some(gpu_model) = line.split(':').nth(2) {
                    // Try to detect if it's NVIDIA, AMD, or Intel
                    let gpu_name = gpu_model.trim();
                    let driver_version = if line_lower.contains("nvidia") {
                        get_nvidia_driver_version()
                    } else if line_lower.contains("amd") || line_lower.contains("radeon") || line_lower.contains("ati") {
                        get_amd_driver_version()
                    } else if line_lower.contains("intel") {
                        get_intel_driver_version()
                    } else {
                        String::from("Unknown")
                    };
                    
                    return (gpu_name.to_string(), driver_version);
                }
            }
        }
    }
    
    // Fallback to other methods if lspci didn't work
    // Check for NVIDIA GPU with nvidia-smi
    if let Ok(output) = Command::new("nvidia-smi").args(&["--query-gpu=name", "--format=csv,noheader"]).output() {
        if !output.stdout.is_empty() {
            let gpu_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return (gpu_name, get_nvidia_driver_version());
        }
    }
    
    // Check for AMD GPU with lshw
    if let Ok(output) = Command::new("lshw").args(&["-C", "display"]).output() {
        let lshw_output = String::from_utf8_lossy(&output.stdout);
        for line in lshw_output.lines() {
            if line.contains("product:") {
                if let Some(product) = line.split(':').nth(1) {
                    let gpu_name = product.trim();
                    return (gpu_name.to_string(), get_amd_driver_version());
                }
            }
        }
    }
    
    (String::from("Unknown GPU"), String::from("Unknown"))
}

fn get_nvidia_driver_version() -> String {
    // Try nvidia-smi first
    if let Ok(output) = Command::new("nvidia-smi").args(&["--query-gpu=driver_version", "--format=csv,noheader"]).output() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return version;
        }
    }
    
    // Try modinfo as fallback
    if let Ok(output) = Command::new("modinfo").args(&["nvidia"]).output() {
        let modinfo_output = String::from_utf8_lossy(&output.stdout);
        for line in modinfo_output.lines() {
            if line.starts_with("version:") {
                if let Some(version) = line.split(':').nth(1) {
                    return version.trim().to_string();
                }
            }
        }
    }
    
    String::from("Unknown")
}

fn get_amd_driver_version() -> String {
    // Check if amdgpu module is loaded
    if Path::new("/sys/module/amdgpu").exists() {
        // First try to check AMDGPU driver version
        if let Ok(output) = Command::new("modinfo").args(&["amdgpu"]).output() {
            let modinfo_output = String::from_utf8_lossy(&output.stdout);
            for line in modinfo_output.lines() {
                if line.starts_with("version:") {
                    if let Some(version) = line.split(':').nth(1) {
                        return format!("AMDGPU {}", version.trim());
                    }
                }
            }
        }
    }
    
    // Check for older Radeon driver
    if Path::new("/sys/module/radeon").exists() {
        if let Ok(output) = Command::new("modinfo").args(&["radeon"]).output() {
            let modinfo_output = String::from_utf8_lossy(&output.stdout);
            for line in modinfo_output.lines() {
                if line.starts_with("version:") {
                    if let Some(version) = line.split(':').nth(1) {
                        return format!("Radeon {}", version.trim());
                    }
                }
            }
        }
    }
    
    // Try to get Mesa version as a fallback
    if let Ok(output) = Command::new("glxinfo").output() {
        let glxinfo_output = String::from_utf8_lossy(&output.stdout);
        for line in glxinfo_output.lines() {
            if line.contains("Mesa") {
                if let Some(idx) = line.find("Mesa") {
                    let mesa_ver = &line[idx..];
                    if let Some(end_idx) = mesa_ver.find('\n') {
                        return mesa_ver[..end_idx].trim().to_string();
                    }
                    return mesa_ver.trim().to_string();
                }
            }
        }
    }
    
    String::from("Unknown")
}

fn get_intel_driver_version() -> String {
    // Try to get i915 driver version
    if Path::new("/sys/module/i915").exists() {
        if let Ok(output) = Command::new("modinfo").args(&["i915"]).output() {
            let modinfo_output = String::from_utf8_lossy(&output.stdout);
            for line in modinfo_output.lines() {
                if line.starts_with("version:") {
                    if let Some(version) = line.split(':').nth(1) {
                        return format!("i915 {}", version.trim());
                    }
                }
            }
        }
    }
    
    // Try to get Mesa version as a fallback
    if let Ok(output) = Command::new("glxinfo").output() {
        let glxinfo_output = String::from_utf8_lossy(&output.stdout);
        for line in glxinfo_output.lines() {
            if line.contains("Mesa") {
                if let Some(idx) = line.find("Mesa") {
                    let mesa_ver = &line[idx..];
                    if let Some(end_idx) = mesa_ver.find('\n') {
                        return mesa_ver[..end_idx].trim().to_string();
                    }
                    return mesa_ver.trim().to_string();
                }
            }
        }
    }
    
    String::from("Unknown")
}

fn display_info(info: &SystemInfo) {
    // Read the fumofetch ASCII art from file
    let logo = read_logo_file().unwrap_or_else(|_| {
        // Fallback logo
        vec![
            "      /\\      ",
            "     /  \\     ",
            "    /\\   \\    ",
            "   /      \\   ",
            "  /   ,,   \\  ",
            " /   |  |   \\ ",
            "/_-''    ''-_\\",
            "             ",
        ].iter().map(|s| s.to_string()).collect()
    });
    
    // Prepare the information lines with proper formatting
    let info_lines = [
        format!("\x1b[1;36m{}@{}\x1b[0m", whoami(), info.hostname),
        format!("\x1b[1;32mOS:\x1b[0m {}", info.os),
        format!("\x1b[1;32mKernel:\x1b[0m {}", info.kernel),
        format!("\x1b[1;32mUptime:\x1b[0m {}", info.uptime),
        format!("\x1b[1;32mShell:\x1b[0m {}", info.shell),
        format!("\x1b[1;32mTerminal:\x1b[0m {}", info.terminal.as_deref().unwrap_or("Unknown")),
        format!("\x1b[1;32mPackages:\x1b[0m {}", info.packages),
        format!("\x1b[1;32mCPU:\x1b[0m {}", info.cpu),
        format!("\x1b[1;32mGPU:\x1b[0m {}", info.gpu),
        format!("\x1b[1;32mGPU Driver:\x1b[0m {}", info.gpu_driver),
        format!("\x1b[1;32mMemory:\x1b[0m {} / {}", info.memory.0, info.memory.1),
    ];
    
    // Print escape sequence to hide cursor and ensure proper display
    print!("\x1b[?25l");
    
    // Calculate the maximum width of the logo for proper alignment
    let max_logo_len = logo.iter().map(|line| visible_length(line)).max().unwrap_or(0);
    let padding = 4;  // Space between logo and info
    
    // Display the logo and info side by side with proper alignment
    for i in 0..logo.len().max(info_lines.len()) {
        let logo_line = if i < logo.len() { &logo[i] } else { "" };
        let info_line = if i < info_lines.len() { &info_lines[i] } else { "" };
        
        // Calculate visible length of the logo line (accounting for ANSI escape sequences)
        let visible_logo_len = visible_length(logo_line);
        let spaces = " ".repeat(max_logo_len - visible_logo_len + padding);
        
        println!("{}{}{}", logo_line, spaces, info_line);
    }
    
    // Show cursor again
    print!("\x1b[?25h");
}

// Helper function to calculate visible length of a string (ignoring ANSI escape sequences)
fn visible_length(s: &str) -> usize {
    let mut visible_len = 0;
    let mut in_escape = false;
    
    for c in s.chars() {
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        
        visible_len += 1;
    }
    
    visible_len
}

fn whoami() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| {
            String::from_utf8_lossy(
                &Command::new("whoami")
                    .output()
                    .unwrap_or_else(|_| panic!("Failed to get username"))
                    .stdout,
            )
            .trim()
            .to_string()
        })
}

fn read_logo_file() -> Result<Vec<String>, std::io::Error> {
    // Try to load from resources directory first
    let resource_path = Path::new("resources").join("fumofetch_logo.txt");
    
    // If resource path exists, try to read it
    let logo_content = if resource_path.exists() {
        fs::read_to_string(resource_path)?
    } else {
        // Fallback to checking in current directory
        let logo_path = Path::new("fumofetch_logo.txt");
        fs::read_to_string(logo_path)?
    };
    
    // Process the logo content line by line, preserving all ANSI escape sequences
    let mut result = Vec::new();
    let mut current_line = String::new();
    
    for c in logo_content.chars() {
        if c == '\n' {
            result.push(current_line);
            current_line = String::new();
        } else {
            current_line.push(c);
        }
    }
    
    // Don't forget the last line if it doesn't end with a newline
    if !current_line.is_empty() {
        result.push(current_line);
    }
    
    Ok(result)
}
