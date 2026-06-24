# Prose, sections, and non-body fences

A file where requests are interleaved with documentation. The parser must
ignore prose, treat non-method H2s and H3s as prose, and only pick up
recognized-language fences as bodies.

## Notes

This is an H2 that is *not* a request — its first word is not an HTTP method,
so it is ordinary prose and must not appear in the request list.

### Background

An H3 stays inside whatever request region precedes it.

## POST /events
Content-Type: application/json

Here is an illustrative shell snippet that must NOT become the body:

```sh
curl https://example.com/events
```

And the real body:

```json
{ "kind": "click" }
```

> assert status == 201

## GET /health

A request with no headers, no body, and no directives.

## PUT /config
X-Form: yes

```form
profile: avatar.png
name: jamie
```
