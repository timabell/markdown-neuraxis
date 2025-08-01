# 1. Choose Dioxus over egui for GUI framework

Date: 2025-08-01

## Status

Accepted

## Context

We need to choose a GUI framework for markdown-neuraxis that supports our long-term vision of cross-platform deployment (Linux/Windows/macOS/Android/iOS) with near-native behavior and performance. The application will be used for note-taking and knowledge management, requiring users to access their notes anywhere, anytime.

Key requirements:
- Pure Rust implementation (non-negotiable constraint)
- Cross-platform desktop support (Linux/Windows/macOS)
- Future mobile support (Android/iOS)  
- Near-native performance and behavior
- Good theming capabilities for user customization
- Largest possible userbase (don't exclude platforms)
- Responsive layouts for different screen sizes

We started with egui for rapid prototyping and proved the core markdown parsing concept works well.

## Decision

We will migrate from egui to Dioxus for the GUI framework.

## Consequences

### Positive
- **Cross-platform by design**: Dioxus desktop renderer is mature, mobile support actively developed
- **Familiar development model**: React-like component architecture with RSX syntax
- **Better theming**: CSS-like styling system much more flexible than egui's immediate mode
- **Responsive design**: Component model naturally supports different screen sizes
- **Future-proof**: Designed from ground up for multi-platform deployment
- **Scalable architecture**: Component-based structure will handle app growth better
- **Parsing logic preserved**: All Rust markdown parsing code (`lib.rs`) stays unchanged

### Negative
- **Migration effort**: Need to rebuild UI layer (but early stage, minimal cost)
- **Ecosystem risk**: Newer framework with smaller community than egui
- **Learning curve**: Different paradigm from egui's immediate mode
- **Mobile still developing**: Mobile renderers not as mature as desktop

### Neutral
- **Pure Rust maintained**: Still 100% Rust codebase
- **Performance**: Both frameworks offer good performance for our use case

## Alternatives Considered

### egui (current)
- ✅ Mature, stable, excellent documentation
- ✅ Immediate mode simplifies state management  
- ✅ Good desktop performance
- ❌ Limited/experimental mobile support
- ❌ Non-native look and feel
- ❌ Theming system not as flexible
- **Verdict**: Good for desktop-only, problematic for mobile future

### Slint
- ✅ Designed for embedded + desktop + mobile
- ✅ Native performance
- ✅ Built-in theming system
- ❌ Smaller community than egui
- ❌ Less familiar development model
- **Verdict**: Strong option but less strategic fit than Dioxus

### Iced
- ✅ Elm-inspired architecture
- ✅ Good performance
- ❌ Mobile support unclear
- ❌ Less mature ecosystem
- **Verdict**: Desktop-focused, mobile story uncertain

### Flutter
- ✅ Excellent mobile + desktop cross-platform support
- ✅ Near-native performance
- ✅ Rich theming and UI component ecosystem
- ✅ Mature mobile development experience
- ❌ Not Rust - would require rewriting all parsing logic in Dart
- ❌ Violates pure Rust constraint
- **Verdict**: Excellent technical choice but ruled out due to constraint

### Tauri + Web Frontend
- ✅ Excellent cross-platform including mobile
- ✅ Rich theming with CSS
- ❌ Not pure Rust (violates hard constraint)
- **Verdict**: Ruled out due to constraint

### Native per platform
- ✅ True native behavior
- ❌ Massive development and maintenance overhead
- **Verdict**: Not feasible for team size

## Implementation Plan

1. Keep all parsing logic in `lib.rs` unchanged
2. Migrate UI layer incrementally:
   - File browser sidebar
   - Markdown outline display  
   - Command line argument handling
   - File loading and selection
3. Test with existing example notes directory
4. Implement CSS-based theming system
5. Plan for responsive design patterns

## Risks and Mitigations

**Risk**: Dioxus mobile support doesn't mature as expected
**Mitigation**: Desktop-first approach, mobile is future enhancement. Can reassess if mobile support stalls.

**Risk**: Breaking changes in newer framework
**Mitigation**: Pin to stable versions, gradual upgrades with testing.

**Risk**: Smaller ecosystem lacks needed functionality
**Mitigation**: Most functionality is in our parsing logic (pure Rust). UI needs are relatively simple.