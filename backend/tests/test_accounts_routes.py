from fastapi.testclient import TestClient
from app.main import app
from app.features.accounts.models import Account
from app.db import get_session
from sqlmodel import text

client = TestClient(app)

created_account_ids = []


def create_account_payload(**overrides):
    """Create a default account payload with optional overrides."""
    payload: Account = {"email": "test@example.com", "google_id": None}
    payload.update(overrides)
    return payload


def record_account_id(response):
    """Record the ID of a created account for cleanup after tests."""
    if response.status_code == 200 and "id" in response.json():
        created_account_ids.append(response.json()["id"])


def test_post_account():
    """Test creating a new account with default payload."""
    response = client.post("/accounts/", json=create_account_payload())
    record_account_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "test@example.com"
    assert data["status"] == "pending"
    assert "id" in data


def test_post_account_minimal_fields():
    """Test creating an account with minimal fields."""
    payload = {"email": "minimal@example.com"}
    response = client.post("/accounts/", json=payload)
    record_account_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "minimal@example.com"
    assert data["status"] == "pending"
    assert data["google_id"] is None


def test_post_account_invalid_email():
    """Test creating an account with an invalid email."""
    response = client.post(
        "/accounts/", json={"email": "invalid-email", "google_id": None}
    )
    assert response.status_code == 422
    detail = response.json()["detail"]
    assert detail[0]["loc"] == ["body", "email"]
    assert "not a valid email address" in detail[0]["msg"]
    assert detail[0]["type"] == "value_error"


def test_post_account_missing_email():
    """Test creating an account with missing email."""
    response = client.post("/accounts/", json={"google_id": None})
    assert response.status_code == 422


def test_post_account_extra_fields():
    """Test creating an account with extra fields."""
    payload = create_account_payload(extra_field="should_be_ignored")
    response = client.post("/accounts/", json=payload)
    record_account_id(response)
    assert response.status_code == 200
    data = response.json()
    assert "extra_field" not in data


def test_post_account_email_null():
    """Test creating an account with email set to None."""
    payload = create_account_payload(email=None)
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 422


def test_post_account_google_id_empty():
    """Test creating an account with google_id as empty string."""
    payload = create_account_payload(google_id="")
    response = client.post("/accounts/", json=payload)
    record_account_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["google_id"] == ""


def test_post_account_google_id_as_int():
    """Test creating an account with google_id as integer."""
    payload = create_account_payload(google_id=123)
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 422


def test_post_account_email_as_int():
    """Test creating an account with email as integer."""
    payload = create_account_payload(email=123)
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 422


def test_get_account():
    """Test retrieving an account by ID."""
    post_resp = client.post(
        "/accounts/",
        json=create_account_payload(email="getme@example.com", google_id="gid123"),
    )
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.get(f"/accounts/{account_id}")
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "getme@example.com"
    assert data["google_id"] == "gid123"


def test_get_account_not_found():
    """Test retrieving a non-existent account."""
    response = client.get("/accounts/999999")  # Assuming this ID does not exist
    assert response.status_code == 404
    assert response.json() == {"detail": "Account not found"}


def test_get_account_invalid_id():
    """Test retrieving an account with an invalid ID."""
    response = client.get("/accounts/invalid_id")
    assert response.status_code == 422  # Unprocessable Entity
    assert (
        "Input should be a valid integer, unable to parse string as an integer"
        in response.json()["detail"][0]["msg"]
    )


def test_get_account_no_id():
    """Test retrieving an account with no ID (should 405)."""
    response = client.get("/accounts/")
    assert response.status_code == 405  # Unprocessable Entity
    assert response.json()["detail"] == "Method Not Allowed"


def test_patch_account():
    """Test patching an account's status."""
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="patchme@example.com")
    )
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"status": "verified"})
    assert response.status_code == 200
    assert response.json()["status"] == "verified"


def test_patch_account_email():
    """Test patching an account's email."""
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="patchmail@example.com")
    )
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(
        f"/accounts/{account_id}", json={"email": "newmail@example.com"}
    )
    assert response.status_code == 200
    assert response.json()["email"] == "newmail@example.com"


def test_patch_account_google_id():
    """Test patching an account's google_id."""
    post_resp = client.post(
        "/accounts/",
        json=create_account_payload(email="patchgid@example.com", google_id="gid1"),
    )
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"google_id": "gid2"})
    assert response.status_code == 200
    assert response.json()["google_id"] == "gid2"


def test_patch_account_no_fields():
    """Test patching an account with no fields (should be no-op)."""
    post_resp = client.post("/accounts/", json=create_account_payload())
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={})
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == account_id


def test_patch_account_invalid_field():
    """Test patching an account with an invalid field (should ignore)."""
    post_resp = client.post("/accounts/", json=create_account_payload())
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"not_a_field": "value"})
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == account_id
    assert "not_a_field" not in data


def test_patch_account_invalid_status():
    """Test patching an account with an invalid status."""
    post_resp = client.post("/accounts/", json=create_account_payload())
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"status": "not_a_status"})
    assert response.status_code == 422


def test_patch_account_email_invalid():
    """Test patching an account with an invalid email."""
    post_resp = client.post("/accounts/", json=create_account_payload())
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"email": "notanemail"})
    assert response.status_code == 422


def test_patch_account_not_found():
    """Test patching a non-existent account."""
    response = client.patch("/accounts/999999", json={"status": "verified"})
    assert response.status_code == 404
    assert response.json() == {"detail": "Account not found"}


def test_delete_account():
    """Test deleting an account."""
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="deleteme@example.com")
    )
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.delete(f"/accounts/{account_id}")
    assert response.status_code == 200
    assert response.json()["ok"] is True
    # Ensure deleted
    get_resp = client.get(f"/accounts/{account_id}")
    assert get_resp.status_code == 404


def test_delete_account_not_found():
    """Test deleting a non-existent account."""
    response = client.delete("/accounts/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "Account not found"}


def test_delete_account_twice():
    """Test deleting an account twice."""
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="twicedelete@example.com")
    )
    record_account_id(post_resp)
    account_id = post_resp.json()["id"]
    response = client.delete(f"/accounts/{account_id}")
    assert response.status_code == 200
    response2 = client.delete(f"/accounts/{account_id}")
    assert response2.status_code == 404


def cleanup_accounts_after_tests(module):
    """Remove all accounts created during the tests."""
    if created_account_ids:
        with next(get_session()) as session:
            ids_tuple = tuple(created_account_ids)
            # Handle single-element tuple for SQL syntax
            if len(ids_tuple) == 1:
                ids_tuple = f"({ids_tuple[0]})"
            session.exec(text(f"DELETE FROM accounts WHERE id IN {ids_tuple}"))
            session.commit()


# Register the cleanup function to run after all tests in this module
teardown_module = cleanup_accounts_after_tests
