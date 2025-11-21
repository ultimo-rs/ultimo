//! Simple, transparent code coverage tool for Ultimo
//!
//! Uses Rust's built-in LLVM instrumentation to generate coverage reports.
//! Minimal dependencies, easy to audit, fully transparent.

use chrono;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    println!("🧪 Ultimo Coverage Tool");
    println!("========================\n");

    let workspace_root = std::env::current_dir().expect("Failed to get current directory");

    // Find LLVM tools
    let llvm_tools_path = find_llvm_tools();
    println!("🔧 Using LLVM tools from: {}", llvm_tools_path.display());

    // Step 1: Clean previous coverage data
    println!("🧹 Cleaning previous coverage data...");
    clean_coverage_data(&workspace_root);

    // Step 2: Set environment variables for coverage instrumentation
    println!("🔧 Setting up coverage instrumentation...");
    let profile_file = workspace_root.join("target/coverage/ultimo-%p-%m.profraw");
    std::env::set_var("CARGO_INCREMENTAL", "0");
    std::env::set_var("RUSTFLAGS", "-C instrument-coverage");
    std::env::set_var("LLVM_PROFILE_FILE", profile_file.to_str().unwrap());

    // Step 3: Run tests with instrumentation
    println!("🧪 Running tests with coverage...\n");
    let test_status = Command::new("cargo")
        .args(&["test", "--lib", "--no-fail-fast"])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-C instrument-coverage")
        .env("LLVM_PROFILE_FILE", profile_file.to_str().unwrap())
        .status()
        .expect("Failed to run tests");

    if !test_status.success() {
        eprintln!("❌ Tests failed!");
        std::process::exit(1);
    }

    println!("\n✅ Tests passed!");

    // Step 4: Find all .profraw files
    println!("📊 Collecting coverage data...");
    let profraw_files: Vec<PathBuf> = WalkDir::new(workspace_root.join("target"))
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "profraw"))
        .map(|e| e.path().to_path_buf())
        .collect();

    if profraw_files.is_empty() {
        eprintln!("❌ No coverage data found!");
        std::process::exit(1);
    }

    println!("   Found {} coverage data files", profraw_files.len());

    // Step 5: Merge profraw files
    println!("🔄 Merging coverage data...");
    let merged_file = workspace_root.join("target/coverage/ultimo.profdata");
    fs::create_dir_all(workspace_root.join("target/coverage"))
        .expect("Failed to create coverage directory");

    let llvm_profdata = llvm_tools_path.join("llvm-profdata");
    let mut merge_cmd = Command::new(&llvm_profdata);
    merge_cmd
        .arg("merge")
        .arg("-sparse")
        .arg("-o")
        .arg(&merged_file);

    for file in &profraw_files {
        merge_cmd.arg(file);
    }

    let merge_status = merge_cmd.status().expect("Failed to run llvm-profdata");

    if !merge_status.success() {
        eprintln!("❌ Failed to merge coverage data!");
        eprintln!("   LLVM tools path: {}", llvm_profdata.display());
        std::process::exit(1);
    }

    // Step 6: Find the test binary
    println!("🔍 Finding test binary...");
    let test_binary = find_test_binary(&workspace_root);

    // Step 7: Generate JSON data for custom HTML report
    println!("📈 Generating coverage data...");
    let html_dir = workspace_root.join("target/coverage/html");
    fs::create_dir_all(&html_dir).expect("Failed to create HTML directory");

    let llvm_cov = llvm_tools_path.join("llvm-cov");

    // Generate JSON export for parsing
    let json_output = Command::new(&llvm_cov)
        .args(&[
            "export",
            &test_binary.to_string_lossy(),
            &format!("-instr-profile={}", merged_file.display()),
            "--ignore-filename-regex=.cargo/registry",
            "--ignore-filename-regex=.rustup/toolchains",
            "--format=text",
        ])
        .output()
        .expect("Failed to run llvm-cov export");

    if !json_output.status.success() {
        eprintln!("❌ Failed to generate coverage data!");
        std::process::exit(1);
    }

    let coverage_data: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("Failed to parse coverage JSON");

    // Step 8: Generate text summary
    println!("📊 Generating summary...");
    let summary_output = Command::new(&llvm_cov)
        .args(&[
            "report",
            &test_binary.to_string_lossy(),
            &format!("-instr-profile={}", merged_file.display()),
            "--ignore-filename-regex=.cargo/registry",
            "--ignore-filename-regex=.rustup/toolchains",
        ])
        .output()
        .expect("Failed to run llvm-cov report");

    let summary = String::from_utf8_lossy(&summary_output.stdout);
    println!("\n{}", summary);

    // Step 8.5: Generate enhanced HTML report
    println!("🎨 Creating modern HTML report...");
    generate_html_report(&workspace_root, &coverage_data, &summary);

    // Step 9: Extract coverage percentage
    let coverage = extract_coverage_percentage(&summary);

    // Step 10: Save results
    let json_report = workspace_root.join("target/coverage/coverage.json");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let json_data = serde_json::json!({
        "coverage_percent": coverage,
        "timestamp": timestamp,
        "test_count": count_tests(&workspace_root),
    });
    fs::write(
        json_report,
        serde_json::to_string_pretty(&json_data).unwrap(),
    )
    .expect("Failed to write JSON report");

    println!("\n✨ Coverage report generated!");
    println!("📄 HTML Report: {}", html_dir.join("index.html").display());
    println!("📊 Coverage: {:.1}%", coverage);

    // Check threshold
    let threshold = 60.0;
    if coverage < threshold {
        eprintln!(
            "\n⚠️  Coverage ({:.1}%) is below minimum threshold ({:.1}%)",
            coverage, threshold
        );
        std::process::exit(1);
    } else {
        println!("\n✅ Coverage meets minimum threshold ({:.1}%)", threshold);
    }
}

fn find_llvm_tools() -> PathBuf {
    // Get the Rust sysroot
    let output = Command::new("rustc")
        .args(&["--print", "sysroot"])
        .output()
        .expect("Failed to get Rust sysroot");

    let sysroot = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Determine the host triple
    let host_triple = if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        "aarch64-apple-darwin"
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") {
        "x86_64-apple-darwin"
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else {
        panic!("Unsupported platform");
    };

    let tools_path = PathBuf::from(sysroot)
        .join("lib/rustlib")
        .join(host_triple)
        .join("bin");

    // Check if llvm-profdata exists
    if !tools_path.join("llvm-profdata").exists() {
        eprintln!("❌ LLVM tools not found!");
        eprintln!("   Expected at: {}", tools_path.display());
        eprintln!("   Run: rustup component add llvm-tools-preview");
        std::process::exit(1);
    }

    tools_path
}

fn clean_coverage_data(workspace_root: &Path) {
    let _ = fs::remove_dir_all(workspace_root.join("target/coverage"));
}

fn find_test_binary(workspace_root: &Path) -> PathBuf {
    let debug_dir = workspace_root.join("target/debug/deps");

    // Find the most recent ultimo test binary
    let mut binaries: Vec<_> = WalkDir::new(&debug_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name.starts_with("ultimo-") && !name.ends_with(".d")
        })
        .collect();

    binaries.sort_by_key(|e| {
        fs::metadata(e.path())
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    binaries
        .last()
        .map(|e| e.path().to_path_buf())
        .expect("No test binary found")
}

fn extract_coverage_percentage(summary: &str) -> f64 {
    // Parse the coverage percentage from llvm-cov output
    // The TOTAL line has multiple columns with percentages
    // We want the line coverage (5th column)
    for line in summary.lines() {
        if line.contains("TOTAL") {
            // Split by whitespace and find percentage values
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Look for the percentage in the line coverage column (typically 4th percentage)
            for part in &parts {
                if part.ends_with('%') && *part != "-" {
                    if let Some(num_str) = part.strip_suffix('%') {
                        if let Ok(percent) = num_str.parse::<f64>() {
                            if percent > 0.0 {
                                // Skip 0.00% placeholders
                                return percent;
                            }
                        }
                    }
                }
            }
        }
    }
    0.0
}

fn count_tests(workspace_root: &Path) -> usize {
    // Count test functions
    let output = Command::new("cargo")
        .args(&["test", "--lib", "--", "--list"])
        .current_dir(workspace_root)
        .output()
        .expect("Failed to list tests");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.ends_with(": test") || line.ends_with(": bench"))
        .count()
}

fn generate_html_report(workspace_root: &Path, _coverage_data: &serde_json::Value, summary: &str) {
    let html_path = workspace_root.join("target/coverage/html/index.html");

    // Parse file coverage from summary - llvm-cov report has this format:
    // Filename    Regions  Missed Regions  Cover  Functions  Missed Functions  Executed  Lines  Missed Lines  Cover
    let mut files = Vec::new();
    let mut in_table = false;
    let mut total_line = String::new();

    for line in summary.lines() {
        if line.contains("Filename") && line.contains("Cover") {
            in_table = true;
            continue;
        }
        if line.contains("----") {
            continue;
        }
        if line.trim().starts_with("TOTAL") {
            total_line = line.to_string();
            break;
        }
        if in_table && !line.trim().is_empty() && line.contains(".rs") {
            // Split by whitespace and extract filename and line coverage percentage
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(filename) = parts.first() {
                if filename.ends_with(".rs") {
                    // Find the line coverage percentage (last percentage before Branches column)
                    // Format: filename regions missed% cover% functions missed% executed% lines missed% COVER% branches...
                    // We want the 9th column (line coverage percentage)
                    if parts.len() >= 10 {
                        let line_coverage = parts[9].to_string();
                        files.push((filename.to_string(), line_coverage));
                    }
                }
            }
        }
    }

    // Sort files by coverage (lowest first)
    files.sort_by(|a, b| {
        let a_val: f64 = a.1.trim_end_matches('%').parse().unwrap_or(0.0);
        let b_val: f64 = b.1.trim_end_matches('%').parse().unwrap_or(0.0);
        a_val.partial_cmp(&b_val).unwrap()
    });

    // Extract total coverage
    let total_coverage = extract_coverage_percentage(&total_line.to_string());

    // Generate file rows HTML
    let file_rows: Vec<String> = files.iter().map(|(name, coverage)| {
        let coverage_num: f64 = coverage.trim_end_matches('%').parse().unwrap_or(0.0);
        let (badge_color, text_color, bg_color) = if coverage_num >= 80.0 {
            ("bg-emerald-100 text-emerald-800 ring-emerald-600/20", "text-emerald-700", "bg-emerald-50")
        } else if coverage_num >= 60.0 {
            ("bg-blue-100 text-blue-800 ring-blue-600/20", "text-blue-700", "bg-blue-50")
        } else if coverage_num >= 40.0 {
            ("bg-amber-100 text-amber-800 ring-amber-600/20", "text-amber-700", "bg-amber-50")
        } else {
            ("bg-red-100 text-red-800 ring-red-600/20", "text-red-700", "bg-red-50")
        };

        format!(r#"
                <tr class="border-b border-gray-100 hover:{} transition-colors">
                  <td class="px-6 py-4 text-sm font-medium text-gray-900">{}</td>
                  <td class="px-6 py-4 text-right">
                    <span class="inline-flex items-center rounded-md px-3 py-1 text-sm font-semibold ring-1 ring-inset {}">
                      {}
                    </span>
                  </td>
                  <td class="px-6 py-4">
                    <div class="w-full bg-gray-200 rounded-full h-2.5">
                      <div class="{} h-2.5 rounded-full transition-all duration-300" style="width: {}"></div>
                    </div>
                  </td>
                </tr>"#, bg_color, name, badge_color, coverage, text_color, coverage)
    }).collect();

    let (overall_badge, overall_bg) = if total_coverage >= 80.0 {
        (
            "bg-emerald-100 text-emerald-800 ring-emerald-600/20",
            "from-emerald-500 to-emerald-600",
        )
    } else if total_coverage >= 60.0 {
        (
            "bg-blue-100 text-blue-800 ring-blue-600/20",
            "from-blue-500 to-blue-600",
        )
    } else if total_coverage >= 40.0 {
        (
            "bg-amber-100 text-amber-800 ring-amber-600/20",
            "from-amber-500 to-amber-600",
        )
    } else {
        (
            "bg-red-100 text-red-800 ring-red-600/20",
            "from-red-500 to-red-600",
        )
    };

    let test_count = count_tests(workspace_root);
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let rust_version = std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "1.91+".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ultimo Coverage Report</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <style>
        @keyframes slideUp {{
            from {{ opacity: 0; transform: translateY(20px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        .animate-slide-up {{
            animation: slideUp 0.5s ease-out forwards;
        }}
        .stagger-1 {{ animation-delay: 0.1s; }}
        .stagger-2 {{ animation-delay: 0.2s; }}
        .stagger-3 {{ animation-delay: 0.3s; }}
    </style>
</head>
<body class="bg-gradient-to-br from-gray-50 to-gray-100 min-h-screen">
    <div class="container mx-auto px-4 py-12 max-w-7xl">
        <!-- Header -->
        <div class="mb-12 animate-slide-up">
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="text-5xl font-bold bg-gradient-to-r {} bg-clip-text text-transparent mb-2">
                        Ultimo Coverage Report
                    </h1>
                    <p class="text-gray-600 text-lg">Generated on {}</p>
                </div>
                <div class="text-right">
                    <div class="inline-flex items-center rounded-full px-6 py-3 text-4xl font-bold ring-2 ring-inset {}">
                        {:.1}%
                    </div>
                    <p class="text-sm text-gray-500 mt-2">Overall Coverage</p>
                </div>
            </div>
        </div>

        <!-- Stats Cards -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
            <div class="bg-white rounded-2xl shadow-lg p-8 border border-gray-200 hover:shadow-xl transition-shadow animate-slide-up stagger-1">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-gray-500 text-sm font-medium mb-2">Total Files</p>
                        <p class="text-4xl font-bold text-gray-900">{}</p>
                    </div>
                    <div class="w-16 h-16 bg-blue-100 rounded-2xl flex items-center justify-center">
                        <svg class="w-8 h-8 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                        </svg>
                    </div>
                </div>
            </div>

            <div class="bg-white rounded-2xl shadow-lg p-8 border border-gray-200 hover:shadow-xl transition-shadow animate-slide-up stagger-2">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-gray-500 text-sm font-medium mb-2">Total Tests</p>
                        <p class="text-4xl font-bold text-gray-900">{}</p>
                    </div>
                    <div class="w-16 h-16 bg-emerald-100 rounded-2xl flex items-center justify-center">
                        <svg class="w-8 h-8 text-emerald-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                        </svg>
                    </div>
                </div>
            </div>

            <div class="bg-white rounded-2xl shadow-lg p-8 border border-gray-200 hover:shadow-xl transition-shadow animate-slide-up stagger-3">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-gray-500 text-sm font-medium mb-2">Threshold</p>
                        <p class="text-4xl font-bold text-gray-900">60%</p>
                    </div>
                    <div class="w-16 h-16 bg-purple-100 rounded-2xl flex items-center justify-center">
                        <svg class="w-8 h-8 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"/>
                        </svg>
                    </div>
                </div>
            </div>
        </div>

        <!-- Coverage Table -->
        <div class="bg-white rounded-2xl shadow-xl border border-gray-200 overflow-hidden animate-slide-up stagger-3">
            <div class="px-8 py-6 bg-gradient-to-r {} border-b border-gray-200">
                <h2 class="text-2xl font-bold text-white">File Coverage Details</h2>
                <p class="text-white/80 mt-1">Line-by-line coverage breakdown</p>
            </div>
            
            <!-- Optional Features Notice -->
            <div class="px-8 py-4 bg-blue-50 border-b border-blue-100">
                <div class="flex items-start space-x-3">
                    <svg class="w-5 h-5 text-blue-600 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    <div class="text-sm">
                        <p class="font-semibold text-blue-900">Note on 0% Coverage Files</p>
                        <p class="text-blue-700 mt-1">
                            Files like <code class="px-1.5 py-0.5 bg-blue-100 rounded text-xs">database/diesel.rs</code>, 
                            <code class="px-1.5 py-0.5 bg-blue-100 rounded text-xs">database/sqlx.rs</code> show 0% because they're 
                            <strong>optional features</strong> not enabled by default. To test these:
                            <code class="ml-2 px-2 py-1 bg-blue-100 rounded text-xs">cargo test --features sqlx-postgres</code>
                        </p>
                    </div>
                </div>
            </div>
            
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-700 uppercase tracking-wider">Filename</th>
                            <th class="px-6 py-4 text-right text-xs font-semibold text-gray-700 uppercase tracking-wider">Coverage</th>
                            <th class="px-6 py-4 text-left text-xs font-semibold text-gray-700 uppercase tracking-wider">Progress</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-100">
                        {}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Footer -->
        <div class="mt-12 text-center text-gray-500 text-sm">
            <p>Generated by <span class="font-semibold text-gray-700">ultimo-coverage</span> • Built with ❤️ for security and transparency</p>
            <p class="mt-2">Using LLVM Coverage Tools • Rust {}</p>
        </div>
    </div>
</body>
</html>"#,
        overall_bg,
        timestamp,
        overall_badge,
        total_coverage,
        files.len(),
        test_count,
        overall_bg,
        file_rows.join("\n"),
        rust_version
    );

    fs::write(html_path, html).expect("Failed to write HTML report");
}
