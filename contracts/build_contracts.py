import configparser
import glob
import os
import subprocess
import sys

# Define absolute paths for contracts and output destination.
CONTRACTS_DIR = os.path.dirname(os.path.abspath(__file__))
WEBSITE_DIR = os.path.abspath(os.path.join(CONTRACTS_DIR, "../website"))
TS_OUT = os.path.join(WEBSITE_DIR, "src", "generated")

def ensure_dir(path: str) -> None:
    """Create the directory if it does not already exist."""
    if not os.path.exists(path):
        os.makedirs(path)

def get_editorconfig_rules() -> dict:
    """Locate and parse the workspace root .editorconfig for [*.proto] rule overrides.
    
    Falls back to official Google Protocol Buffer baseline defaults if the rule
    manifest cannot be resolved or parsed.
    """
    defaults = {"indent_style": "space", "indent_size": 2, "end_of_line": "lf"}
    
    # Traverse upward from current module context to find the project '.editorconfig'.
    cursor = CONTRACTS_DIR
    while cursor and cursor != os.path.dirname(cursor):
        ec_path = os.path.join(cursor, ".editorconfig")
        if os.path.exists(ec_path):
            try:
                parser = configparser.ConfigParser(interpolation=None)
                parser.optionxform = str  # Preserve rule casing.
                parser.read(ec_path, encoding="utf-8")
                
                for section in parser.sections():
                    if "*.proto" in section:
                        sect = parser[section]
                        return {
                            "indent_style": sect.get("indent_style", defaults["indent_style"]),
                            "indent_size": int(sect.get("indent_size", defaults["indent_size"])),
                            "end_of_line": sect.get("end_of_line", defaults["end_of_line"])
                        }
            except Exception:
                break
        cursor = os.path.dirname(cursor)
    return defaults

def process_formatting(fix: bool = False) -> None:
    """Evaluate or auto-correct project .proto file schemas using active .editorconfig parameters.
    
    Guarantees strict, zero-dependency parity across cross-platform developer
    workstations and containerized CI/CD build environments.
    """
    rules = get_editorconfig_rules()
    indent_char = "\t" if rules["indent_style"] == "tab" else " "
    indent_size = 1 if rules["indent_style"] == "tab" else rules["indent_size"]
    eol = "\n" if rules["end_of_line"] == "lf" else "\r\n"
    
    proto_files = glob.glob(os.path.join(CONTRACTS_DIR, "**/*.proto"), recursive=True)
    failed = False
    
    for filepath in proto_files:
        rel_path = os.path.relpath(filepath, CONTRACTS_DIR)
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()
            
        lines = content.splitlines(keepends=True)
        processed_lines = []
        file_mutated = False
        
        for line_num, line in enumerate(lines, 1):
            stripped = line.lstrip(" \t")
            if not stripped.strip():
                processed_lines.append(line)
                continue
                
            # Determine block depth multiplier based on active rule specifications.
            current_indent = len(line) - len(stripped)
            nesting_level = round(current_indent / indent_size) if indent_char == " " else current_indent
            target_indent = (indent_char * indent_size) * nesting_level
            
            normalized_line = target_indent + stripped.rstrip(" \t\r\n") + eol
            
            if line != normalized_line:
                file_mutated = True
                if not fix:
                    print(f"{rel_path}:{line_num} -> Style deviation from .editorconfig constraints.", file=sys.stderr)
                    failed = True
            processed_lines.append(normalized_line)
            
        if fix and file_mutated:
            print(f"Normalizing layout constraints: {rel_path}")
            with open(filepath, "w", encoding="utf-8", newline="") as f:
                f.write("".join(processed_lines))

    if failed and not fix:
        print("\nStyle assertion failed! Run 'just format' to align changes locally.", file=sys.stderr)
        sys.exit(1)
        
    if not fix:
        print("All Protocol Buffer schemas align with current .editorconfig constraints.")

def compile_protos() -> None:
    """Find and compile all Protocol Buffer files into TypeScript source code."""
    ensure_dir(TS_OUT)
    proto_files = glob.glob(os.path.join(CONTRACTS_DIR, "**/*.proto"), recursive=True)
    
    if not proto_files:
        print("No .proto files found.")
        return

    # Automatically handle Windows vs Unix binary extensions for the node module executable.
    plugin_ext = ".cmd" if sys.platform == "win32" else ""
    plugin_path = os.path.join(WEBSITE_DIR, "node_modules", ".bin", f"protoc-gen-ts_proto{plugin_ext}")

    if not os.path.exists(plugin_path):
        print(f"ERROR: ts-proto plugin not found at {plugin_path}", file=sys.stderr)
        print("Please run 'pnpm install' in the website directory first.", file=sys.stderr)
        sys.exit(1)

    print(f"Compiling {len(proto_files)} proto files to TypeScript...")

    cmd = [
        "protoc",
        f"--plugin=protoc-gen-ts_proto={plugin_path}",
        f"--ts_proto_out={TS_OUT}",
        "--ts_proto_opt=esModuleInterop=true,forceLong=long,useOptionals=messages",
        f"-I{CONTRACTS_DIR}"
    ] + proto_files

    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        print("Protoc Compilation Failed:", file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        sys.exit(result.returncode)

    # Automatically prepend @ts-nocheck to bypass strict mode errors in generated files
    generated_ts_files = glob.glob(os.path.join(TS_OUT, "**/*.ts"), recursive=True)
    for ts_file in generated_ts_files:
        with open(ts_file, "r", encoding="utf-8") as f:
            content = f.read()
        
        # Prevent double-writing if the script is run multiple times
        if not content.startswith("// @ts-nocheck"):
            with open(ts_file, "w", encoding="utf-8") as f:
                f.write("// @ts-nocheck\n\n" + content)
    
    print("TypeScript Protobuf Compilation Successful.")

if __name__ == "__main__":
    if "--check-format" in sys.argv:
        process_formatting(fix=False)
        sys.exit(0)
    elif "--format" in sys.argv:
        process_formatting(fix=True)
        sys.exit(0)
        
    compile_protos()
