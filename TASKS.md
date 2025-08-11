# Development Tasks

## In Progress

## Tech tasks

## Bugs
- Main window keeps starting as always-on-top on Linux Mint

## Core MVP Features (Functional Outliner)
- Fix outline parsing hierarchy - children aren't properly nested in current implementation
- Not currently showing markdown content that isn't under a top level bullet
- Replace debug outline display with actual rendered markdown
- Add collapsible bullets with +/- indicators for outline items
- Implement proper indentation handling (tabs vs spaces)
- Add keyboard shortcuts for outline navigation (expand/collapse)

## Wiki Features  
- Implement [[wiki-links]] parsing and rendering
- Add backlink index - scan files for inbound links
- Add link autocomplete when typing [[
- Basic file-to-file navigation

## Task Management
- Parse metadata properties (status:: DOING, due:: date, etc.)
- Implement task state parsing (ACTION, DOING, WAITING, DONE, etc.)
- Add inbox integration - scan for INBOX prefixed bullets
- Simple query system (query:: status:: DOING)

## Daily Workflow
- Journal file integration (journal/ folder support)
- PARA folder structure awareness
- Timeline view for journal entries

## Polish
- Instant write (no save button needed)
- Filesystem watch for external changes
- Dark/light theme switching
- Better error handling and user feedback

## Architecture Improvements Needed
- Outline hierarchy building is flawed - needs rewrite
- Add proper error handling throughout
- Consider state management approach for larger feature set
- Add comprehensive test coverage

## Notes
- Focus on Functional Outliner first - get the core working well
- Each milestone should be fully functional before moving to next
- Test-driven development for all new features
