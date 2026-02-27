#!/usr/bin/env python3
import os
import subprocess
import datetime
import shutil
import glob

DEBUGGER_LOG = "/home/jwils/matrixoverlay.v2/debugger.log"
ARCHIVE_LOG = "/home/jwils/matrixoverlay.v2/debuggerarchive.log"
TRAJECTORY_FILE = "/home/jwils/matrixoverlay.v2/CurrentProgramTrajectory.md"

def run_command(command):
    """Runs a shell command and returns the output formatted for logging."""
    print(f"Running: {command}")
    try:
        result = subprocess.run(
            command,
            shell=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True
        )
        return f"\n$ {command}\n{result.stdout}\n"
    except Exception as e:
        return f"\n$ {command}\nEXECUTION ERROR: {e}\n"

def fix_project_structure():
    """Moves .rs files from root to src/ if found."""
    log_entry = "\n=== Project Structure Fix ===\n"
    
    # Ensure src directory exists
    if not os.path.exists("src"):
        os.makedirs("src")
        log_entry += "Created src/ directory.\n"
    
    # Find .rs files in root
    rs_files = glob.glob("*.rs")
    moved_files = []
    
    for file in rs_files:
        # Don't move build scripts if they exist (usually build.rs)
        if file == "build.rs":
            continue
            
        dest = os.path.join("src", file)
        try:
            shutil.move(file, dest)
            moved_files.append(file)
        except Exception as e:
            log_entry += f"Failed to move {file}: {e}\n"
            
    if moved_files:
        log_entry += f"Moved {len(moved_files)} files to src/: {', '.join(moved_files)}\n"
    else:
        log_entry += "No .rs files found in root to move.\n"
        
    return log_entry

def run_binary():
    """Runs the release binary for a short duration to check for startup errors."""
    command = "./target/release/matrix-overlay"
    print(f"Running binary: {command}")
    log_entry = f"\n=== Runtime Verification ===\n$ RUST_LOG=info {command}\n"
    
    env = os.environ.copy()
    env["RUST_LOG"] = "info"
    
    try:
        # Run with a timeout to catch immediate crashes. 
        # If it runs longer than 5s, we assume it started okay and kill it.
        proc = subprocess.run(
            command,
            shell=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            env=env,
            timeout=5
        )
        # If it exits within timeout (Crash or fast exit)
        log_entry += proc.stdout
        log_entry += f"\n[Process exited with code {proc.returncode}]\n"
        
    except subprocess.TimeoutExpired as e:
        # This is actually good for a long-running app!
        log_entry += e.stdout if e.stdout else ""
        log_entry += "\n[Process ran for 5s (Success). Killed by debugger.]\n"
    except Exception as e:
        log_entry += f"EXECUTION ERROR: {e}\n"
        
    return log_entry

def main():
    timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    # 1. Archive previous log
    if os.path.exists(DEBUGGER_LOG):
        try:
            with open(DEBUGGER_LOG, 'r') as f_src:
                content = f_src.read()
                if content.strip():
                    with open(ARCHIVE_LOG, 'a') as f_dst:
                        f_dst.write(f"\n{'='*40}\nARCHIVED LOG TIMESTAMP: {timestamp}\n{'='*40}\n")
                        f_dst.write(content)
            print(f"Archived previous {DEBUGGER_LOG}")
        except Exception as e:
            print(f"Error archiving log: {e}")

    # 2. Prepare new log content
    log_data = []
    log_data.append(f"DEBUGGER SESSION START: {timestamp}\n")
    
    if os.path.exists(TRAJECTORY_FILE):
        log_data.append(f"Targeting Hypothesis in: {TRAJECTORY_FILE}\n")
    
    # 3. Fix Structure
    log_data.append(fix_project_structure())

    # 4. Run Diagnostics & Build
    log_data.append(run_command("ls -F src/"))
    log_data.append(run_command("ls -F benches/"))
    
    # Build & Test
    log_data.append(run_command("cargo build --release"))
    log_data.append(run_command("cargo test --all-targets"))

    # 5. Runtime Verification
    log_data.append(run_binary())

    # 5. Write to debugger.log
    try:
        with open(DEBUGGER_LOG, 'w') as f:
            f.writelines(log_data)
        print(f"New debug output written to {DEBUGGER_LOG}")
    except Exception as e:
        print(f"Error writing {DEBUGGER_LOG}: {e}")

if __name__ == "__main__":
    main()
