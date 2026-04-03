GET {{base_url}}/movies/{{demo_movie_slug}}/seats
Authorization: Bearer {{token}}
HTTP 200

POST {{base_url}}/bookings/hold
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "movie_slug": "{{demo_movie_slug}}",
  "seat_uuid": "s1"
}
HTTP 201

POST {{base_url}}/bookings/hold
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "movie_slug": "{{demo_movie_slug}}",
  "seat_uuid": "s2"
}
HTTP 201

POST {{base_url}}/bookings/checkout
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "movie_slug": "{{demo_movie_slug}}",
  "seat_uuid": "s2"
}
HTTP 201
