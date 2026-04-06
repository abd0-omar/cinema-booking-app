# Movies API — run against a live server (see AGENTS.md Testing).
# Variables: base_url, token, admin_token (copy hurl/vars.env.example to hurl/vars.env).

# Mutating POST without auth → 401
POST {{base_url}}/movies
HTTP 401

# Authenticated non-admin POST → 403
POST {{base_url}}/movies
Authorization: Bearer {{token}}
Content-Type: application/json

{"title":"Should be forbidden","movie_time":"in 5 min","rows":1,"seats_per_rows":1}
HTTP 403

# Invalid body (empty title) → 422 (admin only)
POST {{base_url}}/movies
Authorization: Bearer {{admin_token}}
Content-Type: application/json

{"title":"","movie_time":"in 5 min","rows":1,"seats_per_rows":1}
HTTP 422

# Create movie → 201, capture slug
POST {{base_url}}/movies
Authorization: Bearer {{admin_token}}
Content-Type: application/json

{"title":"Hurl integration movie","movie_time":"in 5 min","rows":14,"seats_per_rows":22}
HTTP 201
[Captures]
slug: jsonpath "$.slug"
[Asserts]
jsonpath "$.title" == "Hurl integration movie"
jsonpath "$.movie_time" == "in 5 min"
jsonpath "$.rows" == 14
jsonpath "$.seats_per_rows" == 22
jsonpath "$.slug" exists

# Read one (no auth required)
GET {{base_url}}/movies/{{slug}}
HTTP 200
[Asserts]
jsonpath "$.slug" == "{{slug}}"
jsonpath "$.title" == "Hurl integration movie"
jsonpath "$.movie_time" == "in 5 min"
jsonpath "$.rows" == 14

# Update without auth → 401
PUT {{base_url}}/movies/{{slug}}
Content-Type: application/json

{"title":"Should not apply","movie_time":"in 5 min","rows":1,"seats_per_rows":1}
HTTP 401

# Update with admin → 200 (slug recomputed from new title; same id)
PUT {{base_url}}/movies/{{slug}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json

{"title":"Hurl updated movie","movie_time":"in 8 min","rows":16,"seats_per_rows":20}
HTTP 200
[Captures]
slug: jsonpath "$.slug"
[Asserts]
jsonpath "$.title" == "Hurl updated movie"
jsonpath "$.movie_time" == "in 8 min"
jsonpath "$.rows" == 16
jsonpath "$.seats_per_rows" == 20

# Delete without auth → 401
DELETE {{base_url}}/movies/{{slug}}
HTTP 401

# Delete with admin → 204
DELETE {{base_url}}/movies/{{slug}}
Authorization: Bearer {{admin_token}}
HTTP 204

# Gone after delete
GET {{base_url}}/movies/{{slug}}
HTTP 404
