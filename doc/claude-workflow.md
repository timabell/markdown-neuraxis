# Claude Code Development Workflow

This document outlines the standardized development process for markdown-neuraxis using Claude Code with specialized agents.

## Overview

The workflow emphasizes iterative development with automated code review, human oversight, and clear documentation of the development process through commit history.

## When to Use This Workflow

### Use full workflow for:
- New features requiring multiple files
- Complex bug fixes
- Architectural changes
- Any change requiring careful design consideration

### Skip agent workflow for:
- Simple typo fixes
- Single-line changes
- Documentation updates
- Trivial formatting fixes
- Changes that don't require code review

## Step-by-Step Process

### 1. Feature/Bug Discussion & Documentation
- Human discusses the feature or bug with Claude Code
- Create necessary documentation (keep it simple - KISS principle)
- Write ADRs (Architecture Decision Records) for significant architectural choices

### 2. Initiate Development
- Tell Claude to build the feature/fix
- Claude creates a new branch for the change
  - Branch naming: `feature/description`, `fix/description`, `refactor/description`
- Each feature/bug gets its own branch
- Claude uses TodoWrite tool to track implementation tasks

### 3. Code Implementation
- Claude delegates to the **feature-implementor agent** to write the code
  - Full list of prompt(s) needs to be passed to the implementor so it can put an accurate record in the commit
- Agent implements the feature with proper tests
- Agent follows existing code conventions and patterns

### 4. Code Review
- Claude asks the **code-reviewer agent** to review generated code/diff and tests
- Reviewer analyzes:
  - Code quality and design
  - Test coverage and correctness
  - Architectural alignment
  - Security considerations
  - Performance implications

### 5. Human Inspection (MANDATORY STOP)
- **STOP** for human inspection of diff and manual testing
- Human runs code manually to verify functionality
- Human reviews the proposed changes
- Human provides feedback if changes needed

### 6. Iteration Loop
- Claude passes human feedback to implementor agent
- **GOTO Step 3** until code meets standards
- Multiple iterations are acceptable and expected
- Each iteration should show measurable improvement

### 7. Pre-commit Checks
- run ./lint.sh - which run cargo fmt, cargo clippy, and yamllint
- Ensure all tests pass with `cargo test`
- Address any issues before proceeding

### 8. Commit & Documentation
- Claude runs `git add` and `git commit` with clear, descriptive commit message
- Commit message follows project conventions (see CLAUDE.md)
- Includes list of prompts used and co-authorship attribution
- Commits to local git repository (no PR/push unless requested)

## Key Principles

### Branching Strategy
- **One branch per feature/bug**: Each change gets its own branch
- **Atomic commits**: Each commit represents a coherent, logical change
- **Multiple commits OK**: Show the development process including attempts, reviews, and iterations
- **Local only**: All work in local git, no automatic pushes/PRs

### Success Criteria (Ready to Commit)
- ✅ All tests passing (`cargo test`)
- ✅ Code formatted (`cargo fmt`)
- ✅ Clippy warnings addressed (`cargo clippy`)
- ✅ Code review feedback addressed
- ✅ Human approval received
- ✅ TodoWrite tasks completed

### Commit Standards
- Clear, descriptive commit messages
- Include prompt list in commit body
- Co-authorship attribution: `Co-Authored-By: Claude <noreply@anthropic.com>`
- Generated with Claude Code attribution

### Quality Gates
- **Automated**: Code review by specialized agent
- **Manual**: Human inspection and testing (mandatory)
- **Iterative**: Feedback loop until standards met

### Documentation
- ADRs for architectural decisions
- KISS principle - keep documentation simple and focused
- Process documentation in commit messages
- Todo tracking throughout implementation

### Error Handling
- **If something goes wrong**: Stop and ask human for guidance
- **Rollback**: Human decides whether to continue, rollback, or start over
- **Communication**: Claude clearly explains any issues encountered

## Example Workflow

```bash
# 1. Start feature discussion
# Human: "Add support for parsing inline code blocks"

# 2. Claude creates branch
git checkout -b feature/inline-code-blocks

# 3. Feature-implementor agent writes code
# (Implementation with tests)

# 4. Code-reviewer agent reviews
# (Analysis and recommendations)

# 5. Human inspection
# Human tests and provides feedback

# 6. Iteration if needed
# Implementor addresses feedback

# 7. Commit
git add .
git commit -m "feat: Add inline code block parsing support

- Implement pulldown-cmark integration for inline code
- Add comprehensive test coverage for edge cases
- Update ContentBlock enum with InlineCode variant

prompts:
- 'Add support for parsing inline code blocks'
- 'Make sure it handles backticks properly'
- 'Add tests for nested backticks'

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

## Agent Responsibilities

### Feature-Implementor Agent
- Write production-quality code
- Implement comprehensive tests
- Follow existing patterns and conventions
- Consider edge cases and error handling

### Code-Reviewer Agent
- Analyze code quality and design
- Check test coverage and correctness
- Ensure architectural consistency
- Identify potential issues before human review

### Human Developer
- Provide requirements and feedback
- Manual testing and validation
- Final approval of changes
- Strategic architectural decisions

This workflow ensures high code quality while maintaining development velocity and clear documentation of the development process.
