use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};
use colored::*;
use std::io::{self, Write};

pub struct Test {
    args: Vec<String>,
}

impl Test {
    pub fn new(args: Vec<String>) -> Test {
        Test { args }
    }

    pub async fn exec(&self) {
        let test_dir = if self.args.len() > 0 && !self.args[0].starts_with("-") {
            &self.args[0]
        } else {
            "tests"
        };

        if !Path::new(test_dir).exists() {
            println!("{} Directory {} does not exist.", "ERR".red().bold(), test_dir);
            return;
        }

        println!("\n{} Running Ego tests in {}...", " TEST ".on_bright_blue().white().bold(), test_dir);
        println!("{}\n", "=".repeat(50).dimmed());

        let mut tests = vec![];
        let path = PathBuf::from(test_dir);
        if path.is_file() {
            tests.push(path);
        } else {
            self.collect_tests(path, &mut tests);
        }

        if tests.is_empty() {
            println!("{} No .ego files found in {}", "WARN".yellow().bold(), test_dir);
            return;
        }

        let mut passed = 0;
        let mut failed = 0;

        for test_path in tests {
            print!("  {} {} ... ", "RUN".blue(), test_path.display());
            io::stdout().flush().unwrap();

            match self.run_test(&test_path).await {
                Ok(_) => {
                    println!("{}", "PASSED".green().bold());
                    passed += 1;
                }
                Err(e) => {
                    println!("{}", "FAILED".red().bold());
                    println!("{}\n", e.red());
                    failed += 1;
                }
            }
        }

        println!("\n{}", "=".repeat(50).dimmed());
        println!(
            "Test results: {} passed, {} failed",
            passed.to_string().green().bold(),
            failed.to_string().red().bold()
        );

        if failed > 0 {
            std::process::exit(1);
        }
    }

    fn collect_tests(&self, dir: PathBuf, tests: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        self.collect_tests(path, tests);
                    } else if path.extension().map_or(false, |ext| ext == "ego") {
                        tests.push(path);
                    }
                }
            }
        }
    }

    async fn run_test(&self, path: &PathBuf) -> Result<(), String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Parse expectations
        let expectations: Vec<String> = content
            .lines()
            .filter(|line| line.trim().starts_with("// @expect "))
            .map(|line| line.trim().replace("// @expect ", "").trim().to_string())
            .collect();

        // Run the CLI
        // Using `cargo run --quiet -- run <path>` to ensure we use the current source.
        let output = Command::new("cargo")
            .args(["run", "--quiet", "--", "run", path.to_str().unwrap()])
            .output()
            .map_err(|e| format!("Failed to execute cargo: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(format!(
                "Execution failed with status {}\nStdout: {}\nStderr: {}",
                output.status, stdout, stderr
            ));
        }

        // Validate expectations
        let actual_lines: Vec<&str> = stdout.lines().collect();
        
        if expectations.is_empty() {
             // If no expectations, just success is enough
             return Ok(());
        }

        for (i, expected) in expectations.iter().enumerate() {
            if i >= actual_lines.len() {
                return Err(format!("Expected more output. Missing: '{}'", expected));
            }
            if actual_lines[i].trim() != expected {
                return Err(format!(
                    "Output mismatch at line {}\n  expected: '{}'\n  actual:   '{}'\n\nFull Stdout:\n{}",
                    i + 1, expected, actual_lines[i], stdout
                ));
            }
        }

        Ok(())
    }
}
