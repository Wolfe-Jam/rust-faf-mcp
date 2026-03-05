# WJTTC Test Suite — rust-faf-mcp

**Project:** rust-faf-mcp v0.1.0
**Date:** 2026-03-05
**Tester:** WJTTC Championship
**Target:** 95%+ (Championship)

---

## Test Summary

| Tier | Category | Tests | Focus |
|------|----------|-------|-------|
| T1 | BRAKES (Security) | 14 | Path traversal, injection, malformed JSON |
| T2 | ENGINE (Core) | 27 | MCP protocol, 5 tools, language detection |
| T3 | AERO (Edge Cases) | 12 | Unicode, emoji, boundaries, stress |
| **Total** | | **53** | |

---

## Tier 1: BRAKE SYSTEMS (Security)

### T1.1 - Path Traversal Prevention
| Test | Expected | Status |
|------|----------|--------|
| `../../../etc/passwd` as path | Error, no file access | |
| `/etc/shadow` as path | Error, no file written | |
| Path with null bytes | Error | |

### T1.2 - JSON-RPC Injection
| Test | Expected | Status |
|------|----------|--------|
| Malformed JSON input | Skip, no crash | |
| Empty string input | Skip, no crash | |
| Oversized JSON (100KB) | Handle gracefully | |
| Nested JSON depth bomb | Handle gracefully | |

### T1.3 - GitHub URL Injection
| Test | Expected | Status |
|------|----------|--------|
| URL with shell metacharacters | Error, no execution | |
| `javascript:` URL | Error | |
| URL with `..` path traversal | Error | |

### T1.4 - YAML Injection
| Test | Expected | Status |
|------|----------|--------|
| Cargo.toml with YAML injection in name | Escaped safely | |
| Description with quotes/newlines | No YAML break | |
| Project name with shell chars | Escaped | |

---

## Tier 2: ENGINE SYSTEMS (Core)

### T2.1 - MCP Protocol (10 tests)
- initialize handshake
- tools/list returns 5 tools
- tools have schemas
- faf_git has required url
- resources/list
- resources/read weights
- unknown method → error
- unknown tool → error
- JSON-RPC id preserved (int)
- JSON-RPC id preserved (string)

### T2.2 - Tool Functionality (17 tests)
- faf_init creates for Rust project
- faf_init enhances on second run
- faf_init nonexistent directory
- faf_init detects Node
- faf_init detects TypeScript
- faf_init detects Python
- faf_init detects Go
- faf_score no file
- faf_score valid file
- faf_score minimal file
- faf_read no file
- faf_read displays content
- faf_sync creates CLAUDE.md
- faf_sync preserves existing content
- faf_sync no faf file
- faf_git missing url
- faf_git invalid url

---

## Tier 3: AERODYNAMICS (Edge Cases)

### T3.1 - Unicode & Emoji
| Test | Expected | Status |
|------|----------|--------|
| Project name with emoji | Handled | |
| Description with unicode | Parsed correctly | |
| Chinese/Japanese chars in name | Handled | |

### T3.2 - Boundary Scores
| Test | Expected | Status |
|------|----------|--------|
| Empty .faf (version + name only) | Low score, suggestions | |
| Perfect .faf (all fields) | High score | |
| Score displays correct tier badge | All tiers correct | |

### T3.3 - File Edge Cases
| Test | Expected | Status |
|------|----------|--------|
| Empty directory (no manifest) | Creates minimal .faf | |
| Read-only directory | Error, no crash | |
| .faf with extra unknown fields | Parsed, no error | |

### T3.4 - GitHub URL Parsing
| Test | Expected | Status |
|------|----------|--------|
| owner/repo shorthand | Parsed correctly | |
| URL with .git suffix | Stripped | |
| URL with trailing slash | Stripped | |

---

## Run Command

```bash
cargo test
```

## Championship Scoring

| Pass Rate | Tier | Badge |
|-----------|------|-------|
| 95-100% | Championship | 🏆 |
| 85-94% | Podium | 🥇 |
| 70-84% | Points | 🥈 |
| 55-69% | Midfield | 🥉 |
| <55% | DNF | 🔴 |
