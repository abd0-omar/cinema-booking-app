# Movies API — run against a live server (see AGENTS.md Testing).
# Variables: base_url, token (e.g. copy hurl/vars.env.example to hurl/vars.env)

# Mutating POST without auth → 401
POST {{base_url}}/movies
HTTP 401

# Invalid body (empty title) → 422
POST {{base_url}}/movies
Authorization: Bearer {{token}}
Content-Type: application/json

{"title":"","rows":1,"seats_per_rows":1}
HTTP 422

# Create movie → 201, capture slug
POST {{base_url}}/movies
Authorization: Bearer {{token}}
Content-Type: application/json

{"title":"Hurl integration movie","rows":14,"seats_per_rows":22}
HTTP 201
[Captures]
slug: jsonpath "$.slug"
[Asserts]
jsonpath "$.title" == "Hurl integration movie"
jsonpath "$.rows" == 14
jsonpath "$.seats_per_rows" == 22
jsonpath "$.slug" exists

# Read one (no auth required)
GET {{base_url}}/movies/{{slug}}
HTTP 200
[Asserts]
jsonpath "$.slug" == "{{slug}}"
jsonpath "$.title" == "Hurl integration movie"
jsonpath "$.rows" == 14

# Update without auth → 401
PUT {{base_url}}/movies/{{slug}}
Content-Type: application/json

{"title":"Should not apply","rows":1,"seats_per_rows":1}
HTTP 401

# Update with auth → 200 (slug recomputed from new title; same id)
PUT {{base_url}}/movies/{{slug}}
Authorization: Bearer {{token}}
Content-Type: application/json

{"title":"Hurl updated movie","rows":16,"seats_per_rows":20}
HTTP 200
[Captures]
slug: jsonpath "$.slug"
[Asserts]
jsonpath "$.title" == "Hurl updated movie"
jsonpath "$.rows" == 16
jsonpath "$.seats_per_rows" == 20

# Delete without auth → 401
DELETE {{base_url}}/movies/{{slug}}
HTTP 401

# Delete with auth → 204
DELETE {{base_url}}/movies/{{slug}}
Authorization: Bearer {{token}}
HTTP 204

# Gone after delete
GET {{base_url}}/movies/{{slug}}
HTTP 404
