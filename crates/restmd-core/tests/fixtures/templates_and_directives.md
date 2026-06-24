# Templating and directive coverage

Exercises every template modifier and directive shape in one file.

## GET /search?q={{query?}}&page={{page!1}}
Accept: {{accept!application/json}}
X-Request-Id: {{uuid()}}
X-Token: Bearer {{base64(creds)}}

> set retries = 3
> set greeting = hello {{name}}
> capture etag = response.headers.ETag
> capture code = response.status
> capture id   = $.data.id
> assert status >= 200
> assert status != 500
> assert $.items exists
> assert $.name == "Q4 Launch"
> assert $.count >= 3
> assert $.active == true
> assert $.email matches /.+@.+/
