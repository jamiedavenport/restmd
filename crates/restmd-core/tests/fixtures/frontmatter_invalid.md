---
base: https://api.example.com
retries: 2
unknown_key: should be rejected by deny_unknown_fields
---

# Invalid frontmatter still yields requests

The frontmatter has an unknown key, so it fails to parse and is dropped, but the
request below must still be parsed (one error, one request).

## GET /ping
