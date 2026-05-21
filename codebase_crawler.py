import os
from pathlib import Path

# Run with python3 codebase_crawler.py

def generate_codebase_markdown():
    root_dir = Path(__file__).parent.resolve()
    project_name = root_dir.name
    output_filename = f"{project_name}.codebase.md"
    output_path = root_dir / output_filename

    # === IMPROVED CONFIGURATION ===
    supported_extensions = {'.rs', '.py', '.toml', '.md', '.txt', '.json'}

    # Directories to completely skip (add more if needed)
    exclude_dirs = {
        '.git', 'target', 'venv', '__pycache__', '.env',
        'legacy_docs', 'backups', 'docs', 'node_modules', 'dist', 'build'
    }

    # Skip files larger than this (in bytes) to avoid bloat
    MAX_FILE_SIZE = 150_000   # 150 KB

    # Specific files to always skip
    skip_files = {'matrixoverlay.v2.codebase.md', 'codebase_crawler.py'}

    files_processed = 0
    files_skipped_size = 0

    with open(output_path, 'w', encoding='utf-8') as outfile:
        outfile.write(f"# {project_name} Codebase Manifest (Cleaned)\n\n")
        outfile.write(f"**Generated from:** `{root_dir}`\n\n")
        outfile.write("---\n\n")

        for dirpath, dirnames, filenames in os.walk(root_dir):
            dirnames[:] = [d for d in dirnames if d not in exclude_dirs]

            for filename in filenames:
                if filename in skip_files:
                    continue

                file_path = Path(dirpath) / filename
                ext = file_path.suffix.lower()

                if ext in supported_extensions:
                    try:
                        if file_path.stat().st_size > MAX_FILE_SIZE:
                            files_skipped_size += 1
                            continue

                        content = file_path.read_text(encoding='utf-8', errors='ignore')
                        relative_path = file_path.relative_to(root_dir)

                        lang = ext.lstrip('.')
                        outfile.write(f"## {filename}\n")
                        outfile.write(f"**Path:** `{relative_path}`\n\n")
                        outfile.write(f"```{lang}\n{content}\n```\n\n---\n\n")

                        files_processed += 1

                    except Exception as e:
                        outfile.write(f"**FAILED:** `{relative_path}`\nReason: {e}\n\n---\n\n")

    print(f"✅ Done! Created: {output_filename}")
    print(f"   Files included: {files_processed}")
    print(f"   Files skipped (too large): {files_skipped_size}")
    print(f"   Output: {output_path}")

if __name__ == "__main__":
    generate_codebase_markdown()
