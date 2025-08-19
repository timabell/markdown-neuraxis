# Logseq to markdown-neuraxis Converter

Convert Logseq markdown files to markdown-neuraxis format.

## Usage

```bash
python convert.py <logseq_directory>
```

**⚠️ Warning**: This script modifies files in-place. Make a backup first!

## Example

```bash
# Convert Logseq pages in-place
python convert.py ~/logseq/pages

# Or from a Logseq graph export
python convert.py ./logseq-export/pages
```

## Features

### Namespace to Folder Conversion
Converts Logseq's namespace separator (`___`) to real folder structure:

- `projects___website___design.md` → `projects/website/design.md`
- `areas___health___fitness.md` → `areas/health/fitness.md`
- `resources___books___gtd.md` → `resources/books/gtd.md`

Regular files without namespaces are copied as-is:
- `index.md` → `index.md`
- `daily-notes.md` → `daily-notes.md`

## Output

The script will:
1. Create folder structure based on namespaces
2. Move files to appropriate locations in-place
3. Preserve file timestamps
4. Report conversion progress
5. Ask for confirmation before modifying files

## Future Features

Planned enhancements:
- Convert Logseq block references to wiki-links
- Handle journal files (date-based pages)
- Convert Logseq properties to YAML frontmatter
- Process linked references and backlinks
