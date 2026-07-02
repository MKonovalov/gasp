---
name: retry
description: Retry a failed operation with a persisted counter so state survives timeouts.
tools: [bash]
---

# retry

When an operation fails transiently, retry up to 3 times. Persist the retry counter
to disk before each attempt so a timeout mid-retry does not reset the count.
