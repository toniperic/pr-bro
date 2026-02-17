# Configuration Reference

This document covers the full configuration options for PR Bro. For a quick-start guide, see the [README](../README.md).

Configuration file location: `~/.config/pr-bro/config.yaml`

## Full Configuration Example

```yaml
# Theme: "auto" (default, detects terminal), "dark", or "light"
theme: auto

# Auto-refresh interval in seconds (default: 300 = 5 minutes)
auto_refresh_interval: 300

# Global scoring configuration (applies to all queries unless overridden)
scoring:
  base_score: 100
  age: "+1 per 1h"       # Adds 1 point per hour of age
  approvals: "+10 per 1"  # Adds 10 points per approval
  size:
    exclude: ["*.lock", "package-lock.json"]
    buckets:
      - range: "0-9"
        effect: "x10"     # Extremely small PRs get a huge boost
      - range: "10-99"
        effect: "x5"      # Small PRS get a decent boost
      - range: "100-249"
        effect: "x1"      # Medium PRs: no change
      - range: "250-499"
        effect: "x0.5"    # Large PRs get a decent penalty
      - range: ">=500"
        effect: "x0.25"   # Extremely large PRs get a huge penalty
  labels:
    - name: "highest priority"
      effect: "x10"
    - name: "wip"
      effect: "x0.5"
  previously_reviewed: "x2.5"  # Previously reviewed PRs get a boost

# Queries to execute (at least one required)
queries:
  - name: "foo/bar PRs needing my attention"
    query: "is:pr is:open review-requested:@me repository:foo/bar"
    scoring:               # Per-query scoring (merges with global)
      base_score: 50
      age: "x1.5 per 1d"  # Gets a x1.5 boost per day of age
      approvals: "+5 per 1"
```

## Scoring Factors

Each scoring factor is optional and can use addition (`+N`) or multiplication (`xN`) effects.

### Age

Format: `"+N per DURATION"` or `"xN per DURATION"`

Duration uses humantime format: `1h`, `30m`, `1d`, `1w`

Examples:
- `"+1 per 1h"` — adds 1 point per hour of age
- `"x1.1 per 1d"` — multiplies score by 1.1 per day of age

### Approvals

Format: `"+N per 1"`, `"xN per 1"`, `"+N"`, or `"xN"`

This is NOT bucket-based — the effect applies per approval count.

Examples:
- `"+10 per 1"` — adds 10 points per approval
- `"x2 per 1"` — doubles score per approval
- `"+50"` — adds 50 points if any approvals exist

### Size

Bucket-based configuration with optional file exclusions.

**Range formats:**
- `"<N"` — less than N lines
- `"<=N"` — less than or equal to N lines
- `">N"` — greater than N lines
- `">=N"` — greater than or equal to N lines
- `"N-M"` — inclusive range from N to M lines

**Effect formats:**
- `"+N"` — add N points
- `"xN"` — multiply score by N

**Important:** Size bucket ranges must NOT overlap. The validator will reject configurations with overlapping ranges at startup.

Example:

```yaml
size:
  exclude:
    - "*.lock"
    - "package-lock.json"
    - "yarn.lock"
  buckets:
    - range: "<100"
      effect: "x5"      # Small PRs: 5x multiplier
    - range: "100-500"
      effect: "x1"      # Medium PRs: no change
    - range: ">500"
      effect: "x0.5"    # Large PRs: 0.5x penalty
```

**Exclude pattern behavior:**
- Patterns match against the **filename only** (basename), not the full file path. For example, `*.lock` will match `Cargo.lock` and `subdir/package-lock.json`.
- When exclude patterns are configured, PR Bro fetches per-file diff data from the GitHub API to determine which files to exclude. This adds 1-2 API calls per PR (paginated at 100 files per page).
- If the per-file data fetch fails (e.g., rate limit), PR Bro falls back to the aggregate size from the PR summary (no exclusions applied).
- Without exclude patterns, no extra API calls are made.
- Invalid glob patterns are caught at startup during config validation.

### Labels

Optional. Applies score effects based on GitHub labels on the PR. Multiple matching labels compound their effects sequentially (not first-match). Label matching is **case-insensitive**.

```yaml
labels:
  - name: "urgent"
    effect: "+10"     # Add 10 points for urgent PRs
  - name: "wip"
    effect: "x0.5"    # Halve score for work-in-progress
  - name: "critical"
    effect: "x2"      # Double score for critical PRs
```

A PR with both "urgent" and "critical" labels gets both effects: score + 10, then x2.

Each matching label appears as a separate entry in the score breakdown detail view (press `b`).

### Previously Reviewed

Optional. Applies a score effect when the authenticated user (the user whose token is configured) has previously submitted a review on the PR.

If your team is using review requests via GitHub to ask for reviews when a PR is ready to review, this configuration plays well with it, as you can then use `review-requested:@me` filter in the GitHub query to fetch PRs needing your review.

That workflow, paired with this configuration, means that when they need you to re-review such a PR, you could prioritize it using

```yaml
previously_reviewed: "x2.5"   # Prioritize already-reviewed PRs
```

## Effect Syntax Summary

| Syntax | Meaning |
|--------|---------|
| `+N` | Add N points |
| `xN` | Multiply score by N |
| `+N per DURATION` | Add N points per time unit (age only) |
| `xN per DURATION` | Multiply by N per time unit (age only) |
| `+N per M` | Add N points per M units (approvals only) |
| `xN per M` | Multiply by N per M units (approvals only) |

Labels and previously_reviewed use flat effects (`+N` or `xN`), not per-unit effects.

## Per-Query Scoring

Queries can override individual fields of the global scoring configuration. When a PR appears in multiple queries, the **first query's scoring is used** (first-match-wins). Per-query scoring merges with global scoring at the **leaf level** — only the exact sub-fields you specify in a query override the global values; everything else is inherited. This means setting `scoring.size.exclude` in a query does **not** replace the entire `size` block; global `size.buckets` are preserved (and vice versa).

Example:

```yaml
scoring:
  base_score: 100
  age: "+1 per 1h"
  approvals: "+10 per 1"
  size:
    buckets:
      - range: "<100"
        effect: "x5"
      - range: "100-500"
        effect: "x1"
      - range: ">500"
        effect: "x0.5"
  labels:
    - name: "urgent"
      effect: "+20"
    - name: "wip"
      effect: "x0.5"

queries:
  - name: urgent
    query: "is:pr label:urgent"
    scoring:
      age: "+10 per 1h"       # Override: urgent PRs age faster
      size:
        exclude: ["*.lock"]   # Add exclude — inherits global buckets
      labels:
        - name: "urgent"
          effect: "+50"       # Override: stronger urgent boost for this query
      # base_score, approvals — inherited from global
      # size.buckets — inherited from global (not overridden)
      # label "wip" — inherited from global (not mentioned here)

  - name: other
    query: "is:pr org:myorg"
    # No scoring block — uses global scoring entirely
```

In this example, the "urgent" query:
- **Overrides** `age` to `"+10 per 1h"` (urgent PRs age faster).
- **Adds** `size.exclude` with `["*.lock"]`. Because merging is leaf-level, global `size.buckets` are inherited — setting `size.exclude` does NOT replace the entire `size` block.
- **Overrides** the "urgent" label effect from `"+20"` to `"+50"`. Labels merge by name (case-insensitive): the query's "urgent" label wins over the global one. The global "wip" label is preserved because the query does not mention it.
- **Inherits** `base_score`, `approvals`, and `previously_reviewed` from the global config (not specified in the query, so global values apply).

### YAML Merge Keys

YAML merge keys (`<<:`) are supported by the YAML parser for reducing duplication within your config file. This is a YAML feature processed when reading the file, independent of the runtime merge that combines global and per-query scoring. Note that because PR Bro validates config structure strictly (`deny_unknown_fields`), YAML anchors must be placed inside fields that expect the anchored structure, not at the top level. For advanced YAML anchor/merge-key usage, refer to the [YAML specification](https://yaml.org/type/merge.html).

## Theme

PR Bro supports light and dark color themes. The default is `auto`, which detects your terminal's background color at startup and selects the appropriate palette.

```yaml
theme: auto    # Detect terminal background (default)
theme: dark    # Always use dark theme
theme: light   # Always use light theme
```

If auto-detection fails (e.g., over SSH or in tmux), it falls back to the dark theme.

## Config Validation

PR Bro validates your configuration at startup with clear error messages:

- **Unknown YAML keys** are rejected (catches typos like `approvalls` instead of `approvals`)
- **Overlapping size bucket ranges** are rejected (prevents ambiguous scoring)
- **Invalid effect syntax** is caught with helpful messages
- **Empty label names** are rejected
- **Invalid glob patterns** in `size.exclude` are caught (e.g., unclosed character classes like `[invalid`)
- **Invalid label effects** and **invalid previously_reviewed effects** are caught at startup

Validation errors will show exactly what's wrong and where, so you can fix configuration issues quickly.
