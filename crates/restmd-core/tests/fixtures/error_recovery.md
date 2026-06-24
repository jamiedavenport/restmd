---
base: https://api.example.com
defaults:
  Accept: application/json
---

# Malformed inputs

Every request below has something wrong with it, yet the parser should recover
and still produce a request node for each, plus one error per problem.

## GET
Accept: application/json

> assert status == 200

## POST /widgets
Content-Type: application/json

> capture
> capture = $.id
> capture id = somewhere
> assert
> assert status == abc
> assert $.x ~~ 3
> teardown something
> set
> set = 5

## GET /unterminated
Accept: application/json

```json
{ "still": "open"
