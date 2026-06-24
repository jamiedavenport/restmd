---
base: http://127.0.0.1:8787
defaults:
  Accept: application/json
---

# Auth flow

Log in, capture the token and user id, then make an authenticated request that
depends on both. Running the second request will run the login first.

## POST /auth/login
Content-Type: application/json

```json
{ "email": "ada@example.com", "password": "hunter2" }
```

> capture token  = $.access_token
> capture userId = $.user.id
> assert  status == 200
> assert  $.access_token exists

## GET /users/{{userId}}
Authorization: Bearer {{token}}

> assert status == 200
> assert $.id == "u-42"
> assert $.active == true
