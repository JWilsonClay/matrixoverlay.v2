import os
from collections import defaultdict

def generate_codebase_markdown():
    """
    Intelligently finds the workspace root and aggregates files from major programming languages
    into a single markdown file named [projectname].codebase.md.
    Languages are sorted by total code volume (character count) descending.
    Includes visually appealing ASCII art separators for better readability.
    """
    # Automatically determine the workspace root (where the script is placed)
    root_dir = os.path.dirname(os.path.abspath(__file__))
    project_name = os.path.basename(root_dir)
    output_filename = f"{project_name}.codebase.md"
    output_path = os.path.join(root_dir, output_filename)

    # Exclude directories that typically contain non-project/binary/dependency code
    exclude_dirs = {'.git', 'venv', '__pycache__', '.env', '.pytest_cache', 'env', 'node_modules', '.idea', 'target', 'build', 'dist'}

    # Define major programming languages and their file extensions
    lang_extensions = {
        'Python': ['.py'],
        'Rust': ['.rs'],
        'JavaScript': ['.js', '.jsx'],
        'TypeScript': ['.ts', '.tsx'],
        'Java': ['.java'],
        'C++': ['.cpp', '.hpp', '.cc', '.cxx', '.h', '.hh'],
        'C': ['.c', '.h'],
        'Go': ['.go'],
        'Ruby': ['.rb'],
        'PHP': ['.php'],
        'Swift': ['.swift'],
        'Kotlin': ['.kt'],
        'Scala': ['.scala'],
        'HTML': ['.html', '.htm'],
        'CSS': ['.css', '.scss', '.sass'],
        'Shell': ['.sh', '.bash'],
        'C#': ['.cs'],
        'R': ['.r', '.R'],
        'SQL': ['.sql'],
        # Add more as needed
    }

    # Invert to map extension to language
    ext_to_lang = {}
    for lang, exts in lang_extensions.items():
        for ext in exts:
            ext_to_lang[ext.lower()] = lang

    # Collect files by language
    files_by_lang = defaultdict(list)
    for dirpath, dirnames, filenames in os.walk(root_dir):
        # Prune excluded directories to focus on project source
        dirnames[:] = [d for d in dirnames if d not in exclude_dirs]
        
        for filename in filenames:
            # Get extension
            _, ext = os.path.splitext(filename)
            ext = ext.lower()
            if ext not in ext_to_lang:
                continue  # Not a supported language
            
            # Exclude this script itself
            if filename == os.path.basename(__file__):
                continue
                
            file_path = os.path.join(dirpath, filename)
            lang = ext_to_lang[ext]
            
            try:
                with open(file_path, 'r', encoding='utf-8', errors='ignore') as infile:
                    content = infile.read()
                files_by_lang[lang].append((filename, dirpath, content))
            except Exception as e:
                # Skip problematic files
                pass

    # Compute volume (total characters) for each language
    lang_volumes = {}
    for lang, files in files_by_lang.items():
        total_chars = sum(len(content) for _, _, content in files)
        lang_volumes[lang] = total_chars

    # Sort languages by volume descending
    sorted_langs = sorted(lang_volumes.keys(), key=lambda l: lang_volumes[l], reverse=True)

    # Helper function to create centered ASCII box
    def create_ascii_box(text, width=40, double_line=False):
        if double_line:
            top_char, bottom_char, horiz, vert = '╔', '╚', '═', '║'
            top_right, bottom_right = '╗', '╝'
        else:
            top_char, bottom_char, horiz, vert = '┌', '└', '─', '│'
            top_right, bottom_right = '┐', '┘'
        
        text = text.upper()
        padding = width - len(text) - 2  # 2 for borders
        left_pad = padding // 2
        right_pad = padding - left_pad
        
        top = top_char + horiz * (width - 2) + top_right + '\n'
        middle = vert + ' ' * left_pad + text + ' ' * right_pad + vert + '\n'
        bottom = bottom_char + horiz * (width - 2) + bottom_right + '\n'
        return top + middle + bottom

    # Write to output
    with open(output_path, 'w', encoding='utf-8') as outfile:
        # Project overview box
        outfile.write(create_ascii_box("Project Codebase Overview", width=40, double_line=False))
        
        # Main header
        outfile.write(f"# {project_name} Codebase Manifest\n\n")
        
        # Nice graphical separator (double line horizontal)
        separator_width = 40
        outfile.write('═' * separator_width + '\n\n')
        
        for lang in sorted_langs:
            # Language box (double line for distinction)
            outfile.write(create_ascii_box(lang, width=40, double_line=True))
            
            for filename, dirpath, content in files_by_lang[lang]:
                outfile.write(f"{filename}\n")
                outfile.write(f"{dirpath}\n")
                outfile.write(f"```{lang.lower()}\n")
                outfile.write(content)
                outfile.write("\n```\n\n")
                # Visual separator between files (simple dashed line)
                outfile.write('-' * 80 + '\n\n')
            
            # Extra space between languages
            outfile.write("\n\n")

    print(f"Success: '{output_filename}' generated in workspace root: {root_dir}")

if __name__ == "__main__":
    generate_codebase_markdown()
