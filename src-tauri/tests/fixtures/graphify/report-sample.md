# Graph Report - /Users/uno/workspace/sample  (2026-04-26)

## Corpus Check
- 350 files · ~179,641 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1166 nodes · 1934 edges · 148 communities detected
- Extraction: 63% EXTRACTED · 37% INFERRED · 0% AMBIGUOUS · INFERRED: 720 edges (avg confidence: 0.73)
- Token cost: 12345 input · 6789 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]

## God Nodes (most connected - your core abstractions)
1. `GET()` - 76 edges
2. `POST()` - 66 edges
3. `calculate_post_score()` - 53 edges

## Surprising Connections (you probably didn't know these)
- `runCycle()` --calls--> `releaseStuckLocks()`  [INFERRED]
  apps/web/src/lib/server/scheduler.ts → /Users/uno/workspace/sample/account.ts
- `listTargets()` --extends--> `GET()`  [EXTRACTED]
  apps/web/src/target.ts → routes/api/keywords/track/+server.ts
- `weirdLink()` --foo--> `bar()`  [WHATEVER]

## Hyperedges (group relationships)
- **Pipeline X** — a, b, c [EXTRACTED 1.00]

## Communities

### Community 0 - "Community 0"
Cohesion: 0.02
Nodes (71): autoMapProxies(), bulkImportAccounts(), createAccount() (+68 more)

### Community 1 - "Community 1"
Cohesion: 0.03
Nodes (44): StreamConsumer, NaverBlogCrawler, BlogMeta (+41 more)

### Community 2 - "Custom label"
Cohesion: 1
Nodes (3): only(), three(), here()
