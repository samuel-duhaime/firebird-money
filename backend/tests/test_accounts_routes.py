from fastapi.testclient import TestClient
from src.main import app

client = TestClient(app)


def test_post_account():
    response = client.post(
        "/accounts/", json={"email": "test@example.com", "google_id": None}
    )
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "test@example.com"
    assert data["status"] == "pending"


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
