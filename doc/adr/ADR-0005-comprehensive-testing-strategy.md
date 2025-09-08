# ADR-0005: Comprehensive Testing Strategy for Multi-Frontend Architecture

**Status:** Proposed  
**Date:** 2025-09-08  
**Deciders:** Tim Abell

## Context

### The Problem: Scaling Regression Testing Across Multiple Frontends

The project aims to support multiple UI frontends (Dioxus desktop, web, TUI, mobile apps, native mobile) while maintaining high confidence that GitHub Actions CI can prevent regressions across all shipped features and frontends. The goal is: **"I can review any PR, and if the GitHub actions pass then it is highly unlikely to regress/break any previously shipped features or frontends."**

### Current Challenge: The Multiple Textarea Bug

A persistent bug demonstrates the core testing challenge: clicking on nested list items causes multiple textareas to appear simultaneously, indicating an editing state management issue likely rooted in anchor ID collision. This bug manifests in the Dioxus desktop and would appear in any frontend using the same core document logic.

### Research: Dioxus Testing Recommendations

Based on Dioxus official documentation ([Testing Guide](https://dioxuslabs.com/learn/0.6/cookbook/testing/)), the framework recommends a layered testing approach:

1. **Component Testing**: Using `dioxus_ssr` with `assert_rsx_eq!` for rendering validation
2. **Hook Testing**: Custom `VirtualDom` driving for state transition testing  
3. **End-to-End Testing**: Playwright integration for full workflow validation

**Reference:** [GPT Chat Discussion on Dioxus Testing](https://chatgpt.com/share/68bf4ccf-474c-8006-a31a-819fd8b590ce) exploring various testing approaches.

### Outside-In Testing Philosophy Alignment

Per Tim's preference for outside-in testing ([Why Do Automated Tests Matter?](https://0x5.uk/2024/03/27/why-do-automated-tests-matter/)), the ideal approach starts with real user behavior and works down to unit tests for efficiency and specificity.

### The Combinatorial Explosion Problem

With N frontends × M features × P platforms, full E2E testing becomes unsustainable:

```
❌ Explosive: Test every feature on every frontend
✅ Layered: Test core once, UI contracts, frontend specifics
```

## Decision

### Comprehensive Testing Architecture

We will implement a **Core-First Testing Strategy** with targeted integration boundary testing:

```
┌─────────────────────────────────────┐
│ COMPREHENSIVE CORE TESTING (95%)    │
│ - Document manipulation logic       │
│ - Anchor ID management              │
│ - Markdown parsing & rendering      │
│ - File system operations            │
│ - Property-based testing (future)   │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│ UI CONTRACT TESTING (Interface)     │
│ - Core ↔ UI command boundaries      │
│ - State synchronization             │
│ - Mock-based integration tests      │
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│ FRONTEND-SPECIFIC TESTING (Minimal) │
│ - Input method differences          │
│ - Platform-specific behaviors       │
│ - Critical path smoke tests         │
└─────────────────────────────────────┘
```

### Implementation Phases

#### Phase 1: Fix Current Integration Bug (Immediate)
- Create targeted integration test reproducing multiple textarea bug
- Implement minimal testing infrastructure to pin down root cause
- Fix anchor ID collision issue at integration boundary
- Establish pattern for future integration boundary tests

#### Phase 2: Core Testing Foundation (Next)
- Comprehensive core logic test suite
- Anchor uniqueness invariant testing
- Document state consistency testing
- Cross-platform compatibility testing

#### Phase 3: Contract Testing Layer (Future)
- UI command interface contracts
- State synchronization verification
- Mock-based boundary testing
- Backend integration contracts

#### Phase 4: Property-Based Testing (YAGNI for now)
- Fuzz testing for edge cases
- Generative test scenarios
- Performance regression detection

### GitHub Actions Strategy

```yaml
jobs:
  core-comprehensive:
    # Must pass for any frontend to be safe
    - Core logic tests (comprehensive)
    - Integration boundary tests
    - Anchor uniqueness verification
    
  ui-contracts:
    # Verify core↔UI boundary contracts
    - Command interface contracts
    - State synchronization tests
    
  frontend-behavior:
    # Only frontend-specific differences
    matrix: [dioxus-web, dioxus-desktop, future-frontends]
    - Input method tests
    - Critical path smoke tests
```

### Key Principles

1. **Integration Boundary Focus**: Test where bugs actually occur (core↔UI boundary)
2. **Avoid Combinatorial Explosion**: Core tests don't multiply by frontend count
3. **Fast Feedback**: Most regressions caught in fast core tests
4. **Contract Stability**: UI frontends can evolve independently if contracts maintained
5. **Real User Impact First**: Outside-in approach starting with user-visible behaviors

## Consequences

### Positive
- **Scalable**: Adding new frontends doesn't explode test suite
- **Fast CI**: Core tests provide quick feedback on most regressions  
- **High Confidence**: Comprehensive core coverage prevents most bugs
- **Maintainable**: Clear separation between core, contract, and frontend tests
- **Cost-Effective**: Focuses testing effort on highest-impact areas

### Negative
- **Initial Setup Overhead**: Establishing contracts and boundaries requires upfront work
- **Integration Boundary Complexity**: Requires careful design of core↔UI contracts
- **Potential Blind Spots**: Some frontend-specific integration issues might be missed
- **Contract Maintenance**: Interface contracts need updating as core evolves

### Risks and Mitigations

**Risk**: Integration boundary bugs slip through despite good core/UI tests  
**Mitigation**: Targeted integration tests for critical user workflows

**Risk**: Contract tests become stale as interfaces evolve  
**Mitigation**: Automated contract validation as part of core API changes

**Risk**: Frontend-specific behaviors inadequately tested  
**Mitigation**: Critical path smoke tests for each shipped frontend

## Implementation Notes

### Immediate Action: Multiple Textarea Bug
- Create integration test that reproduces user scenario
- Test loads document with nested bullets
- Simulate focus state changes via UI interactions
- Assert exactly one textarea visible at any time
- Fix root cause (suspected anchor ID collision in UI component lookup)

### Testing Infrastructure Needed
- Basic integration test framework
- Document creation helpers for consistent test data
- UI component testing utilities (minimal Dioxus VirtualDom usage)
- CI integration for automated regression detection

## References

- [Dioxus Testing Documentation](https://dioxuslabs.com/learn/0.6/cookbook/testing/)
- [Outside-In Testing Philosophy](https://0x5.uk/2024/03/27/why-do-automated-tests-matter/)
- [GPT Discussion on Dioxus Testing Approaches](https://chatgpt.com/share/68bf4ccf-474c-8006-a31a-819fd8b590ce)

## Related ADRs

- ADR-0001: GUI Framework Choice (Dioxus selection enables this testing strategy)
- ADR-0002: Plugin Architecture (Affects testing boundaries for extensibility)
- ADR-0004: Editing Code Architecture (Defines core↔UI contracts for testing)
