use crate::domain::models::OutlineItem;

/// Build a hierarchical outline from flat list of outline items
pub fn build_hierarchy(items: Vec<OutlineItem>) -> Vec<OutlineItem> {
    let mut result = Vec::new();
    let mut stack: Vec<OutlineItem> = Vec::new();
    let mut pending_children: Vec<OutlineItem> = Vec::new();

    for item in items {
        // If this is a child item, store it for later
        if item.level > 0 {
            pending_children.push(item);
            continue;
        }

        // This is a top-level item - add any pending children to the last parent
        if let Some(mut parent) = stack.pop() {
            parent.children = pending_children;
            pending_children = Vec::new();
            result.push(parent);
        }

        stack.push(item);
    }

    // Handle the last parent with any remaining children
    if let Some(mut parent) = stack.pop() {
        parent.children = pending_children;
        result.push(parent);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_flat_hierarchy() {
        let items = vec![
            OutlineItem::new("Item 1".to_string(), 0),
            OutlineItem::new("Item 2".to_string(), 0),
        ];

        let result = build_hierarchy(items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "Item 1");
        assert_eq!(result[1].content, "Item 2");
    }

    #[test]
    fn test_build_nested_hierarchy() {
        let items = vec![
            OutlineItem::new("Parent".to_string(), 0),
            OutlineItem::new("Child 1".to_string(), 1),
            OutlineItem::new("Child 2".to_string(), 1),
        ];

        let result = build_hierarchy(items);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Parent");
        assert_eq!(result[0].children.len(), 2);
        assert_eq!(result[0].children[0].content, "Child 1");
        assert_eq!(result[0].children[1].content, "Child 2");
    }
}
