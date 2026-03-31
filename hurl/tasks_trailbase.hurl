# End-to-end: Trailbase password login → JWT → cinema-booking /tasks.
#
# Prerequisites:
# - Trailbase running at {{trailbase_url}} (same instance that issued the JWT signing key).
# - cinema-booking running with APP_TRAILBASE__JWT_PUBLIC_KEY_PATH or APP_TRAILBASE__JWT_PUBLIC_KEY_PEM
#   set to that instance’s Ed25519 public PEM (not APP_ENVIRONMENT=test / not test-helpers).
# - Trailbase credentials: fresh local DB uses admin@localhost with a random password in server logs,
#   not necessarily {{trailbase_password}} — use `trail user change-password` or see vars.trailbase.env.example.
# - Account must not require MFA (otherwise login returns 403).
# - If registration requires email verification, use an already-verified user.
#
# Variables: see hurl/vars.trailbase.env.example

POST {{trailbase_url}}/api/auth/v1/login
Content-Type: application/json

{"email":"{{trailbase_email}}","password":"{{trailbase_password}}"}
HTTP 200
[Captures]
auth_token: jsonpath "$.auth_token"
[Asserts]
jsonpath "$.auth_token" exists
jsonpath "$.refresh_token" exists

# Mutating API must accept the Trailbase-issued auth JWT
POST {{api_base_url}}/tasks
Authorization: Bearer {{auth_token}}
Content-Type: application/json

{"description":"Hurl task via Trailbase JWT"}
HTTP 201
[Captures]
task_uuid: jsonpath "$.uuid"
[Asserts]
jsonpath "$.description" == "Hurl task via Trailbase JWT"

GET {{api_base_url}}/tasks/{{task_uuid}}
HTTP 200
[Asserts]
jsonpath "$.uuid" == "{{task_uuid}}"

DELETE {{api_base_url}}/tasks/{{task_uuid}}
Authorization: Bearer {{auth_token}}
HTTP 204

GET {{api_base_url}}/tasks/{{task_uuid}}
HTTP 404
