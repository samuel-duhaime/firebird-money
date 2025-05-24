from fastapi.testclient import TestClient
from app.main import app
from app.db import init_db

init_db()  # Ensure tables are created before running tests

client = TestClient(app)


# TODO: Add more tests for edge cases and error handling for all the methods
def test_post_account():
    response = client.post(
        "/accounts/", json={"email": "test@example.com", "google_id": None}
    )
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "test@example.com"
    assert data["status"] == "pending"


def test_post_account_invalid_email():
    response = client.post(
        "/accounts/", json={"email": "invalid-email", "google_id": None}
    )
    assert response.status_code == 422
    detail = response.json()["detail"]
    assert detail[0]["loc"] == ["body", "email"]
    assert "not a valid email address" in detail[0]["msg"]
    assert detail[0]["type"] == "value_error"


def test_get_account():
    # Create account first
    post_resp = client.post(
        "/accounts/", json={"email": "getme@example.com", "google_id": "gid123"}
    )
    account_id = post_resp.json()["id"]
    response = client.get(f"/accounts/{account_id}")
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "getme@example.com"
    assert data["google_id"] == "gid123"


def test_get_account_not_found():
    response = client.get("/accounts/999999")  # Assuming this ID does not exist
    assert response.status_code == 404
    assert response.json() == {"detail": "Account not found"}


def test_get_account_invalid_id():
    response = client.get("/accounts/invalid_id")
    assert response.status_code == 422  # Unprocessable Entity
    assert (
        "Input should be a valid integer, unable to parse string as an integer"
        in response.json()["detail"][0]["msg"]
    )


def test_get_account_no_id():
    response = client.get("/accounts/")
    assert response.status_code == 405  # Unprocessable Entity
    assert response.json()["detail"] == "Method Not Allowed"


def test_patch_account():
    # Create account first
    post_resp = client.post(
        "/accounts/", json={"email": "patchme@example.com", "google_id": None}
    )
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"status": "verified"})
    assert response.status_code == 200
    assert response.json()["status"] == "verified"


def test_delete_account():
    # Create account first
    post_resp = client.post(
        "/accounts/", json={"email": "deleteme@example.com", "google_id": None}
    )
    account_id = post_resp.json()["id"]
    response = client.delete(f"/accounts/{account_id}")
    assert response.status_code == 200
    assert response.json()["ok"] is True
    # Ensure deleted
    get_resp = client.get(f"/accounts/{account_id}")
    assert get_resp.status_code == 404
