# User-visible API strings

transaction-not-found = No transaction with id { $n }
category-not-found = No category with id { $n }
category-duplicate-name = A category with this name already exists
category-invalid-type = type must be one of income, expense, or transfer
category-in-use = Category { $n } is still used by existing transactions
household-not-found = No household with id { $n }
household-in-use = Household { $n } still has members connected to it
user-not-found = No user with id { $n }
user-duplicate-email = A user with this email already exists
user-invalid-status = status must be one of verified, pending, or suspended
user-in-use = User { $n } is still connected to a household
household-member-not-found = No household member with id { $n }
household-member-duplicate = This user is already connected to this household
household-member-invalid-type = type must be family_manager or family_member
auth-email-invalid = A valid email address is required
auth-token-invalid = This sign-in link is invalid, expired, or already used
auth-email-send-failed = The sign-in email could not be sent, please try again
auth-not-signed-in = You must be signed in
auth-join-code-not-found = No household matches this join code
auth-already-in-household = You are already connected to this household
import-job-not-found = No import job found
import-file-required = A file to import is required
import-file-too-large = The uploaded file is too large (10 MB max)
internal-db-error = Internal server error
