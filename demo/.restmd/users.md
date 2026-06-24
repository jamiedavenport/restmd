---
base: http://127.0.0.1:8787
defaults:
  Accept: application/json
---

# Users

CRUD-ish requests against the demo server. The last request fails its assertion
on purpose, to show how a failure looks.

## GET /users

> assert status == 200
> assert $[0].id == "u-1"

## POST /users
Content-Type: application/json

```json
{ "name": "Grace" }
```

> assert status == 201
> capture newId = $.id

## GET /users/u-7

> assert status == 200
> assert $.score >= 3

## GET /status/404

> assert status == 200
