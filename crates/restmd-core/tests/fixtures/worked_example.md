---
base: https://api.{{env}}.example.com
openapi: ./openapi.yaml
environments:
  dev:
    env: dev-api
    workspace_id: ws_dev_abc123
  prod:
    env: api
    workspace_id: ws_prod_xyz789
defaults:
  Accept: application/json
  User-Agent: restmd/0.1
  Authorization: Bearer {{token}}
---

# Project Management API

Auth flow first, then CRUD on projects within a workspace.

## POST /auth/login
Content-Type: application/json

```json
{
  "email": "{{email}}",
  "password": "{{password}}"
}
```

> capture token  = $.access_token
> capture userId = $.user.id
> assert  status == 200
> assert  $.access_token exists

## GET /workspaces/{{workspace_id}}/projects?status=active&limit=50

## POST /workspaces/{{workspace_id}}/projects
Content-Type: application/json
Idempotency-Key: {{uuid()}}

```json
{ "name": "Q4 Launch", "members": ["{{userId}}"] }
```

> capture projectId = $.id

## DELETE /workspaces/{{workspace_id}}/projects/{{projectId}}

> assert status == 204
