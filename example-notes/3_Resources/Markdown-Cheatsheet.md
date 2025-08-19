# Markdown Cheatsheet

*Quick reference for markdown syntax supported in markdown-neuraxis*

## Basic Syntax

### Headings
```markdown
# H1 - Main Title
## H2 - Section
### H3 - Subsection  
#### H4 - Topic
##### H5 - Subtopic
###### H6 - Detail
```

### Text Formatting
```markdown
**Bold text**
*Italic text*
***Bold and italic***
~~Strikethrough~~
`Inline code`
```

### Line Breaks
```markdown
End a line with two spaces  
for a line break

Or use a blank line

for a paragraph break
```

**Note:** Two spaces at the end of a line create a line break. A blank line creates a paragraph break.

## Lists

### Unordered Lists
```markdown
- First item
- Second item
  - Nested item (2 spaces or tab)
  - Another nested
    - Even deeper
- Back to top level

* Can also use asterisks
+ Or plus signs
```

### Ordered Lists
```markdown
1. First item
2. Second item
   1. Nested numbered
   2. Another nested
3. Third item
```

### Task Lists
```markdown
- [ ] Uncompleted task
- [x] Completed task
- [ ] Another todo
```

## Links

### Wiki Links (Internal)
```markdown
[[Getting-Started]]
[[journal/2024-01-15]]
[[1_Projects/Website-Redesign]]
```

### Markdown Links
```markdown
[Link text](https://example.com)
[Link with title](https://example.com "Title text")
```

### Reference Links
```markdown
[Link text][1]
[Another link][example]

[1]: https://example.com
[example]: https://example.org
```

## Images
```markdown
![Alt text](image.jpg)
![Alt text](image.jpg "Title")
![Local image](assets/screenshot.png)
```

## Code

### Inline Code
```markdown
Use `const` instead of `let` in JavaScript
```

### Code Blocks
````markdown
```javascript
function hello() {
  console.log("Hello, World!");
}
```

```python
def hello():
    print("Hello, World!")
```

```bash
echo "Hello, World!"
```
````

### Code Block with Line Numbers
````markdown
```rust {linenos=true}
fn main() {
    println!("Hello, World!");
}
```
````

## Blockquotes
```markdown
> This is a blockquote
> It can span multiple lines

> Blockquotes can be nested
>> Like this
>>> And even deeper
```

## Tables
```markdown
| Header 1 | Header 2 | Header 3 |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |

# Alignment
| Left | Center | Right |
|:-----|:------:|------:|
| A    |   B    |     C |
```

## Horizontal Rules
```markdown
Three or more...

---
Hyphens

***
Asterisks

___
Underscores
```

## Metadata Properties

### In Bullets
```markdown
- Task with metadata
  status:: active
  priority:: high
  due:: 2024-01-20
  assigned:: @john
```

### Inline Properties
```markdown
- ACTION:: Do something important due::2024-01-20
- Meeting with client:: Acme Corp at:: 2pm location:: Zoom
```

## Task States (GTD)
```markdown
- INBOX:: Unprocessed item
- ACTION:: Concrete next action
- DOING:: Currently working on
- WAITING:: Blocked on something
- DONE:: Completed task
- SOMEDAY:: Maybe later
```

## Special Syntax

### Tags
```markdown
#productivity #markdown #reference
```


### Contexts (GTD)
```markdown
#@home #@office #@phone #@computer #@errands
```


## Advanced Features

### Nested Bullets with Mixed Content
```markdown
- Main point
  - Sub point with **bold**
    ```python
    # Code in a bullet
    print("Hello")
    ```
  - Another sub point
    > Quote in a bullet
    - Even deeper nesting
```

### Metadata Blocks
```yaml
---
title: Document Title
date: 2024-01-15
tags: [markdown, reference]
status: draft
---
```

### HTML (When Needed)
```html
<details>
<summary>Click to expand</summary>

Hidden content here

</details>
```

## Best Practices

1. **Consistent formatting** - Pick a style and stick to it
2. **Meaningful headers** - Use descriptive section titles
3. **Link liberally** - Connect related ideas with [[wiki-links]]
4. **Semantic lists** - Use task lists for todos, bullets for notes
5. **Code language** - Always specify language for syntax highlighting
6. **Alt text** - Always include for images (accessibility)


---

*For more examples, see [[Getting-Started]] and explore the journal entries*