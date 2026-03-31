# Tasks API — run against a live server (see README Testing).
# Variables: base_url, token (e.g. hurl/vars.env from vars.env.example)

# Mutating POST without auth → 401
POST {{base_url}}/tasks
HTTP 401

# Invalid body (empty description) → 422
POST {{base_url}}/tasks
Authorization: Bearer {{token}}
Content-Type: application/json

{"description":""}
HTTP 422

# Create task → 201, capture uuid
POST {{base_url}}/tasks
Authorization: Bearer {{token}}
Content-Type: application/json

{"description":"Hurl integration task"}
HTTP 201
[Captures]
uuid: jsonpath "$.uuid"
[Asserts]
jsonpath "$.description" == "Hurl integration task"
jsonpath "$.uuid" exists

# Read one (no auth required)
GET {{base_url}}/tasks/{{uuid}}
HTTP 200
[Asserts]
jsonpath "$.uuid" == "{{uuid}}"
jsonpath "$.description" == "Hurl integration task"

# Update without auth → 401
PUT {{base_url}}/tasks/{{uuid}}
Content-Type: application/json

{"description":"Should not apply"}
HTTP 401

# Update with auth → 200
PUT {{base_url}}/tasks/{{uuid}}
Authorization: Bearer {{token}}
Content-Type: application/json

{"description":"Hurl updated task"}
HTTP 200
[Asserts]
jsonpath "$.uuid" == "{{uuid}}"
jsonpath "$.description" == "Hurl updated task"

# Delete without auth → 401
DELETE {{base_url}}/tasks/{{uuid}}
HTTP 401

# Delete with auth → 204
DELETE {{base_url}}/tasks/{{uuid}}
Authorization: Bearer {{token}}
HTTP 204

# Gone after delete
GET {{base_url}}/tasks/{{uuid}}
HTTP 404

# Batch create (PUT /tasks) → 201
PUT {{base_url}}/tasks
Authorization: Bearer {{token}}
Content-Type: application/json

[{"description":"Hurl batch A"},{"description":"Hurl batch B"}]
HTTP 201
[Asserts]
jsonpath "$" count == 2
jsonpath "$[0].description" == "Hurl batch A"
jsonpath "$[1].description" == "Hurl batch B"
