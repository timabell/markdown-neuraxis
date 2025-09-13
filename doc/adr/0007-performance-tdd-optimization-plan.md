# ADR-0007: Performance TDD Optimization Plan for UI Delays

## Status
Proposed

## Context

The Dioxus UI is experiencing multi-second delays when loading and editing non-trivial markdown files, making the application unusable for real-world scenarios. A comprehensive codebase investigation has identified critical performance bottlenecks that compound to create these delays.

### Performance Investigation Findings

The primary performance issues identified are:

#### Critical Issues (Multi-second impact)
1. **Excessive Document Cloning** - Every UI component receives full `Document` clones that trigger complete tree-sitter re-parsing (`document.rs:490-514`)
2. **Missing UI Component Caching** - No memoization in UI components, causing complete re-render cascades for large documents (`document_content.rs:20-30`)
3. **Synchronous Tree-sitter Operations** - All parsing blocks the main thread (`document.rs:120-138`)
4. **Auto-save on Every Keystroke** - Synchronous file I/O on every edit (`app.rs:159-170`)

#### High Impact Issues
- **Inefficient String Operations** - Multiple unnecessary `rope.to_string()` conversions (`document.rs:264, 443`)
- **Anchor System Overhead** - Complex anchor rebinding operations on every edit (`anchors.rs:68-100`)

#### Missing Benchmark Coverage
Current benchmarks only test isolated operations with tiny test data, missing:
- UI component render performance
- Document cloning in UI context
- Multi-operation edit sequences (actual typing scenarios)
- File I/O + parsing pipeline integration
- Snapshot creation under UI load

### Root Cause Analysis

The multi-second delays stem from architectural anti-patterns:
- **Heavy cloning**: `document: document.clone()` throughout UI components
- **Synchronous operations**: Tree-sitter parsing blocks UI thread
- **Cascading updates**: Document changes trigger full re-computation of all dependent components
- **Missing incremental processing**: Every edit treated as full document re-processing

## Decision

We will implement a **Performance Test-Driven Development (TDD)** approach to systematically address these bottlenecks. Each optimization will be driven by specific performance benchmarks that define success criteria.

## Performance TDD Implementation Plan

### Phase 1: Performance Test Infrastructure (TDD Foundation)

**Goal**: Establish performance benchmarks that fail with current implementation, then drive optimizations to make them pass.

#### 1.1 Critical UI Pipeline Benchmarks
- **Benchmark**: Document loading + rendering pipeline (target: <50ms for 1000-line files)
- **Benchmark**: Edit sequence simulation (target: <16ms per keystroke for 60fps)
- **Benchmark**: Component re-render overhead (target: <5ms for large documents)
- **Benchmark**: Memory allocation pressure during editing sessions

#### 1.2 Integration Benchmarks
- **Benchmark**: File I/O + parsing + UI update full pipeline
- **Benchmark**: Multi-file switching performance
- **Benchmark**: Document clone operations in UI context

### Phase 2: Document Sharing Architecture (Highest Impact)

**TDD Approach**: Write benchmarks showing current document cloning overhead, then implement Arc-based sharing until benchmarks pass.

#### 2.1 Replace Document Cloning with Arc<Document>
- **Target**: Eliminate `document.clone()` calls in UI components
- **Implementation**: `Arc<Document>` + interior mutability pattern
- **Success Criteria**: Benchmark shows >90% reduction in parsing overhead

#### 2.2 Implement Copy-on-Write Document State
- **Target**: Share immutable document state across components
- **Implementation**: CoW wrapper around document operations
- **Success Criteria**: Memory allocation benchmark shows dramatic reduction

### Phase 3: UI Component Optimization (User-Visible Impact)

**TDD Approach**: Benchmark component render times, then add memoization until render performance targets are met.

#### 3.1 Component Memoization Layer
- **Target**: Prevent unnecessary re-renders of unchanged content
- **Implementation**: Dioxus `use_memo` for expensive renders
- **Success Criteria**: Large document render benchmark <100ms

#### 3.2 Intelligent Component Keying
- **Target**: Stable component identity across updates
- **Implementation**: Content-hash based keys instead of index-based
- **Success Criteria**: Edit benchmark shows minimal DOM thrashing

### Phase 4: Async Architecture (Threading)

**TDD Approach**: Write benchmarks showing main thread blocking, then implement async parsing until main thread stays responsive.

#### 4.1 Background Tree-sitter Parsing
- **Target**: Keep main thread responsive during parsing
- **Implementation**: Tokio async parsing with incremental updates
- **Success Criteria**: Main thread never blocks >16ms during parsing

#### 4.2 Debounced Operations
- **Target**: Batch rapid operations efficiently
- **Implementation**: Auto-save debouncing, parse debouncing
- **Success Criteria**: Keystroke latency benchmark <5ms average

### Phase 5: Incremental Processing (Advanced Optimizations)

**TDD Approach**: Benchmark current full-reprocessing overhead, then implement incremental algorithms.

#### 5.1 Incremental Snapshot Diffing
- **Target**: Only recompute changed document sections
- **Implementation**: Tree diffing for snapshot updates
- **Success Criteria**: Large document edit benchmark shows O(1) updates

#### 5.2 Optimized Anchor System
- **Target**: Process only affected ranges during edits
- **Implementation**: Range-based anchor rebinding
- **Success Criteria**: Anchor processing benchmark scales linearly

## TDD Methodology

### Red-Green-Refactor Cycle
1. **Red**: Write failing performance benchmark showing current bottleneck
2. **Green**: Implement minimal fix to make benchmark pass
3. **Refactor**: Clean up implementation while maintaining performance gains

### Performance Targets
- **File Loading**: <100ms for 10MB markdown files
- **Keystroke Latency**: <16ms (60fps responsive)
- **Memory Growth**: <10MB per hour of editing
- **Component Render**: <50ms for 1000+ block documents

### Benchmark Categories
- **Micro**: Individual operation performance (existing)
- **Integration**: Multi-component pipeline performance (new)
- **Macro**: Real-world usage scenarios (new)
- **Regression**: Ensure optimizations don't break existing functionality

## Implementation Priority

1. **Phase 1** (Foundation) - Essential for measuring progress
2. **Phase 2** (Document Sharing) - Highest impact on current bottlenecks
3. **Phase 3** (UI Optimization) - Most visible user experience improvements
4. **Phase 4** (Async Architecture) - Required for responsive editing
5. **Phase 5** (Incremental Processing) - Advanced optimizations for scale

## Consequences

### Positive
- **Systematic approach**: Each optimization is measurable and targeted
- **No regressions**: Benchmarks ensure existing functionality isn't broken
- **Scalable performance**: Architecture will handle larger documents gracefully
- **User experience**: Sub-second response times for all operations
- **Maintainable**: Performance characteristics are codified in tests

### Negative
- **Development overhead**: Writing benchmarks before optimization takes time
- **Architecture changes**: Some refactoring required to support new patterns
- **Complexity**: Async and incremental processing adds system complexity

### Risks
- **Premature optimization**: Must ensure benchmarks target actual user pain points
- **Over-engineering**: Balance between performance and code simplicity
- **Platform differences**: Performance characteristics may vary across platforms

## Implementation Notes

### Critical Performance Code Paths
- **File loading**: `crates/markdown-neuraxis-dioxus/src/ui/app.rs:36-58`
- **Edit handling**: `crates/markdown-neuraxis-dioxus/src/ui/app.rs:147-176`
- **Component rendering**: `crates/markdown-neuraxis-dioxus/src/ui/components/document_content.rs:17-33`
- **Document operations**: `crates/markdown-neuraxis-engine/src/editing/document.rs:98-169`

### Benchmark Infrastructure Location
- New benchmarks will be added to `crates/markdown-neuraxis-engine/benches/`
- UI-specific benchmarks may require new benchmark crate in `crates/markdown-neuraxis-dioxus/benches/`

This TDD approach ensures each optimization addresses real bottlenecks with measurable improvements while maintaining system reliability and user experience quality.