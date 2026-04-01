# End-to-end: Trailbase password login → JWT → cinema-booking /movies.
#
# Prerequisites:
# - Trailbase running at {{trailbase_url}} (same instance that issued the JWT signing key).
# - cinema-booking running with APP_TRAILBASE__JWT_PUBLIC_KEY_PATH or APP_TRAILBASE__JWT_PUBLIC_KEY_PEM
#   set to that instance’s Ed25519 public PEM (not APP_ENVIRONMENT=test / not test-helpers).
# - Trailbase credentials: see vars.trailbase.env.example (password may need aligning with `trail user change-password`).
# - Account must not require MFA (otherwise login returns 403).
# - Movie POST/PUT/DELETE require a Trailbase admin user (JWT `admin: true`); non-admins get 403.
#
# Variables: see hurl/vars.trailbase.env.example (same as tasks_trailbase.hurl)

POST {{trailbase_url}}/api/auth/v1/login
Content-Type: application/json

{"email":"{{trailbase_email}}","password":"{{trailbase_password}}"}
HTTP 200
[Captures]
auth_token: jsonpath "$.auth_token"
[Asserts]
jsonpath "$.auth_token" exists
jsonpath "$.refresh_token" exists

# Mutating POST without auth → 401
POST {{api_base_url}}/movies
HTTP 401

# Invalid body (empty title) → 422
POST {{api_base_url}}/movies
Authorization: Bearer {{auth_token}}
Content-Type: application/json

{"title":"","rows":1,"seats_per_rows":1}
HTTP 422

# Create movie → 201, capture slug
POST {{api_base_url}}/movies
Authorization: Bearer {{auth_token}}
Content-Type: application/json

{"title":"Hurl Trailbase movie","rows":14,"seats_per_rows":22}
HTTP 201
[Captures]
slug: jsonpath "$.slug"
[Asserts]
jsonpath "$.title" == "Hurl Trailbase movie"
jsonpath "$.rows" == 14
jsonpath "$.seats_per_rows" == 22
jsonpath "$.slug" exists

# Read one (no auth required)
GET {{api_base_url}}/movies/{{slug}}
HTTP 200
[Asserts]
jsonpath "$.slug" == "{{slug}}"
jsonpath "$.title" == "Hurl Trailbase movie"
jsonpath "$.rows" == 14

# Update without auth → 401
PUT {{api_base_url}}/movies/{{slug}}
Content-Type: application/json

{"title":"Should not apply","rows":1,"seats_per_rows":1}
HTTP 401

# Update with auth → 200 (slug recomputed from new title; same id)
PUT {{api_base_url}}/movies/{{slug}}
Authorization: Bearer {{auth_token}}
Content-Type: application/json

{"title":"Hurl Trailbase movie updated","rows":16,"seats_per_rows":20}
HTTP 200
[Captures]
slug: jsonpath "$.slug"
[Asserts]
jsonpath "$.title" == "Hurl Trailbase movie updated"
jsonpath "$.rows" == 16
jsonpath "$.seats_per_rows" == 20

# Delete without auth → 401
DELETE {{api_base_url}}/movies/{{slug}}
HTTP 401

# Delete with auth → 204
DELETE {{api_base_url}}/movies/{{slug}}
Authorization: Bearer {{auth_token}}
HTTP 204

# Gone after delete
GET {{api_base_url}}/movies/{{slug}}
HTTP 404
