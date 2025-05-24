from fastapi.testclient import TestClient
from app.main import app
from app.db import init_db

init_db()  # Ensure tables are created before running tests

client = TestClient(app)


def create_account_payload(**overrides):
    payload = {"email": "test@example.com", "google_id": None}
    payload.update(overrides)
    return payload


def test_post_account():
    response = client.post("/accounts/", json=create_account_payload())
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "test@example.com"
    assert data["status"] == "pending"
    assert "id" in data


def test_post_account_minimal_fields():
    payload = {"email": "minimal@example.com"}
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["email"] == "minimal@example.com"
    assert data["status"] == "pending"
    assert data["google_id"] is None


def test_post_account_invalid_email():
    response = client.post(
        "/accounts/", json={"email": "invalid-email", "google_id": None}
    )
    assert response.status_code == 422
    detail = response.json()["detail"]
    assert detail[0]["loc"] == ["body", "email"]
    assert "not a valid email address" in detail[0]["msg"]
    assert detail[0]["type"] == "value_error"


def test_post_account_missing_email():
    response = client.post("/accounts/", json={"google_id": None})
    assert response.status_code == 422


def test_post_account_extra_fields():
    payload = create_account_payload(extra_field="should_be_ignored")
    response = client.post("/accounts/", json=payload)
    assert response.status_code in (200, 422)


def test_post_account_email_null():
    payload = create_account_payload(email=None)
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 422


def test_post_account_google_id_empty():
    payload = create_account_payload(google_id="")
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["google_id"] == ""


def test_post_account_google_id_as_int():
    payload = create_account_payload(google_id=123)
    response = client.post("/accounts/", json=payload)
    assert response.status_code in (200, 422)


def test_post_account_email_as_int():
    payload = create_account_payload(email=123)
    response = client.post("/accounts/", json=payload)
    assert response.status_code == 422


def test_get_account():
    post_resp = client.post(
        "/accounts/",
        json=create_account_payload(email="getme@example.com", google_id="gid123"),
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
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="patchme@example.com")
    )
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"status": "verified"})
    assert response.status_code == 200
    assert response.json()["status"] == "verified"


def test_patch_account_email():
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="patchmail@example.com")
    )
    account_id = post_resp.json()["id"]
    response = client.patch(
        f"/accounts/{account_id}", json={"email": "newmail@example.com"}
    )
    assert response.status_code == 200
    assert response.json()["email"] == "newmail@example.com"


def test_patch_account_google_id():
    post_resp = client.post(
        "/accounts/",
        json=create_account_payload(email="patchgid@example.com", google_id="gid1"),
    )
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"google_id": "gid2"})
    assert response.status_code == 200
    assert response.json()["google_id"] == "gid2"


def test_patch_account_no_fields():
    post_resp = client.post("/accounts/", json=create_account_payload())
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={})
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == account_id


def test_patch_account_invalid_field():
    post_resp = client.post("/accounts/", json=create_account_payload())
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"not_a_field": "value"})
    assert response.status_code in (200, 422)


def test_patch_account_invalid_status():
    post_resp = client.post("/accounts/", json=create_account_payload())
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"status": "not_a_status"})
    assert response.status_code == 422


def test_patch_account_email_invalid():
    post_resp = client.post("/accounts/", json=create_account_payload())
    account_id = post_resp.json()["id"]
    response = client.patch(f"/accounts/{account_id}", json={"email": "notanemail"})
    assert response.status_code == 422


def test_patch_account_not_found():
    response = client.patch("/accounts/999999", json={"status": "verified"})
    assert response.status_code == 404
    assert response.json() == {"detail": "Account not found"}


def test_delete_account():
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="deleteme@example.com")
    )
    account_id = post_resp.json()["id"]
    response = client.delete(f"/accounts/{account_id}")
    assert response.status_code == 200
    assert response.json()["ok"] is True
    # Ensure deleted
    get_resp = client.get(f"/accounts/{account_id}")
    assert get_resp.status_code == 404


def test_delete_account_not_found():
    response = client.delete("/accounts/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "Account not found"}


def test_delete_account_twice():
    post_resp = client.post(
        "/accounts/", json=create_account_payload(email="twicedelete@example.com")
    )
    account_id = post_resp.json()["id"]
    response = client.delete(f"/accounts/{account_id}")
    assert response.status_code == 200
    response2 = client.delete(f"/accounts/{account_id}")
    assert response2.status_code == 404
