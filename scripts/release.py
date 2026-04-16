#!/usr/bin/env python3

import sys
import os
import re
import subprocess
from datetime import date

def run(cmd, capture=True, env = {}):
    print(f"+ {cmd}")
    result = subprocess.run(cmd, shell=True, capture_output=capture, text=True,
                            env=os.environ | env)
    if result.returncode != 0:
        print(f"Command failed with exit code {result.returncode}")
        if capture:
            print(result.stdout)
            print(result.stderr)
        sys.exit(1)
    return result.stdout

def update_cargo_toml(version):
    with open("Cargo.toml", "r") as f:
        content = f.read()
    
    new_content = re.sub(r'^version = ".*?"', f'version = "{version}"', content, count=1, flags=re.MULTILINE)
    
    with open("Cargo.toml", "w") as f:
        f.write(new_content)

def update_changelog(version):
    with open("CHANGELOG.md", "r") as f:
        lines = f.readlines()
    
    today = date.today().isoformat()
    new_lines = []
    
    for line in lines:
        if line.startswith("## [Unreleased]"):
            new_lines.append(line)
            new_lines.append("\n")
            new_lines.append(f"## [{version}] - {today}\n")
            continue
        
        new_lines.append(line)
    
    # Update links at the bottom
    # [Unreleased]: https://github.com/fowles/pratdiff/compare/3.1.1...main
    # [3.1.1]: https://github.com/fowles/pratdiff/compare/3.1.0...3.1.1
    
    unreleased_link_re = re.compile(r"^\[Unreleased\]: (.*?/compare/)(.*?)\.\.\.main")
    
    for i in range(len(new_lines)):
        m = unreleased_link_re.match(new_lines[i])
        if m:
            base_url = m.group(1)
            old_prev = m.group(2)
            new_lines[i] = f"[Unreleased]: {base_url}{version}...main\n"
            # Insert the new version link below it
            new_version_link = f"[{version}]: {base_url}{old_prev}...{version}\n"
            if new_version_link not in new_lines:
                new_lines.insert(i + 1, new_version_link)
            break
            
    with open("CHANGELOG.md", "w") as f:
        f.writelines(new_lines)

def update_readme():
    help_output = run("cargo run --quiet -- --help", env = {"COLUMNS": "80"})
    
    with open("README.md", "r") as f:
        content = f.read()
    
    start_marker = "❯ pratdiff --help\n"
    end_marker = "\n```"
    
    start_idx = content.find(start_marker)
    if start_idx == -1:
        print("Could not find usage start marker in README.md")
        return

    end_idx = content.find(end_marker, start_idx)
    if end_idx == -1:
        print("Could not find usage end marker in README.md")
        return
        
    new_content = content[:start_idx + len(start_marker)] + help_output.rstrip() + content[end_idx:]
    
    with open("README.md", "w") as f:
        f.write(new_content)

def main():
    if len(sys.argv) != 2:
        print("Usage: release.py <VERSION>")
        sys.exit(1)
    
    version = sys.argv[1]
    
    # Simple check for version format (e.g. 1.2.3)
    if not re.match(r"^\d+\.\d+\.\d+$", version):
        print(f"Warning: version '{version}' does not look like a standard semver (x.y.z)")
        return
    
    print(f"Releasing version {version}...")
    
    update_cargo_toml(version)
    update_changelog(version)
    update_readme()
    
    print("Next steps:")
    print("1. commit the local changes")
    print("2. git tag", version)
    print("3. git push --tags")
    print("4. cargo publish")

if __name__ == "__main__":
    main()
