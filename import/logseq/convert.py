#!/usr/bin/env python3
"""
Convert Logseq markdown files to markdown-neuraxis format in-place.

Features:
- Convert ___ namespace separators to real folders (foo___bar.md -> foo/bar.md)
"""

import os
import shutil
import sys
from pathlib import Path


def convert_namespace_to_folders(directory):
    """
    Convert Logseq namespace files (using ___) to folder structure in-place.

    Args:
        directory: Path to directory containing Logseq files
    """
    dir_path = Path(directory)

    if not dir_path.exists():
        print(f"Error: Directory '{directory}' does not exist")
        return False

    converted_count = 0

    # Process all markdown files with namespaces
    for md_file in dir_path.glob("*.md"):
        filename = md_file.stem  # filename without extension

        # Skip files without namespaces
        if "___" not in filename:
            continue

        # Split by namespace separator and create folder structure
        parts = filename.split("___")

        # Create nested folder structure
        folder_path = dir_path
        for part in parts[:-1]:  # All parts except the last become folders
            folder_path = folder_path / part
            folder_path.mkdir(exist_ok=True)

        # Last part becomes the filename
        new_file_path = folder_path / f"{parts[-1]}.md"

        print(f"Converting: {md_file.name} -> {new_file_path.relative_to(dir_path)}")

        # Move the file to new location
        shutil.move(str(md_file), str(new_file_path))
        converted_count += 1

    print(f"\nConverted {converted_count} files in-place")
    return True


def main():
    """Main entry point."""
    if len(sys.argv) != 2:
        print("Usage: python convert.py <logseq_directory>")
        print("\nExample:")
        print("  python convert.py ~/logseq/pages")
        print("  python convert.py ./exported-pages")
        sys.exit(1)

    directory = sys.argv[1]

    print(f"Converting Logseq format in: {directory}")
    print("WARNING: This will modify files in-place!")
    print()

    # Ask for confirmation
    response = input("Continue? (y/N): ").lower().strip()
    if response not in ['y', 'yes']:
        print("Cancelled.")
        sys.exit(0)

    if convert_namespace_to_folders(directory):
        print("\nConversion complete!")
    else:
        print("\nConversion failed!")
        sys.exit(1)


if __name__ == "__main__":
    main()
