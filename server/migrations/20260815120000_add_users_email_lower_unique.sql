-- The `email` UNIQUE constraint on `users` is case-sensitive, so "Sam@x.com" and "sam@x.com"
-- could otherwise become two separate accounts. Every write path already normalizes to lowercase
-- (`normalize_email` on sign-in) except direct `POST /users`; this closes the gap at the database
-- level regardless of caller, and gives `get_by_email`'s `lower(email) = lower($1)` lookup an
-- index to use instead of a sequential scan.
CREATE UNIQUE INDEX users_email_lower_key ON users (lower(email));
