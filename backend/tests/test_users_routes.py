from fastapi.testclient import TestClient
from app.main import app
from app.db import init_db

init_db()  # Ensure tables are created before running tests

client = TestClient(app)


def create_user_payload(**overrides):
    payload = {
        "account_id": 1,
        "type": "family_manager",
        "first_name": "John",
        "last_name": "Doe",
        "logo_url": "http://example.com/logo.png",
    }
    payload.update(overrides)
    return payload


def test_post_user():
    response = client.post("/users/", json=create_user_payload())
    assert response.status_code == 200
    data = response.json()
    assert data["account_id"] == 1
    assert data["type"] == "family_manager"
    assert data["first_name"] == "John"
    assert data["last_name"] == "Doe"
    assert data["logo_url"] == "http://example.com/logo.png"
    assert "id" in data


def test_post_user_minimal_fields():
    payload = {"account_id": 1, "type": "family_member"}
    response = client.post("/users/", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["account_id"] == 1
    assert data["type"] == "family_member"
    assert data["first_name"] is None
    assert data["last_name"] is None
    assert data["logo_url"] is None


def test_post_user_missing_account_id():
    payload = {"type": "family_manager"}
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_missing_type():
    payload = {"account_id": 1}
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_invalid_type():
    payload = create_user_payload(type="not_a_type")
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_extra_fields():
    payload = create_user_payload(extra_field="should_be_ignored")
    response = client.post("/users/", json=payload)
    assert response.status_code in (200, 422)


def test_post_user_logo_url_null():
    payload = create_user_payload(logo_url=None)
    response = client.post("/users/", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["logo_url"] is None


def test_post_user_logo_url_empty():
    payload = create_user_payload(logo_url="")
    response = client.post("/users/", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["logo_url"] == ""


def test_post_user_first_name_empty():
    payload = create_user_payload(first_name="")
    response = client.post("/users/", json=payload)
    assert response.status_code == 200


def test_post_user_last_name_empty():
    payload = create_user_payload(last_name="")
    response = client.post("/users/", json=payload)
    assert response.status_code == 200


def test_post_user_account_id_as_string():
    payload = create_user_payload(account_id="1")
    response = client.post("/users/", json=payload)
    assert response.status_code in (200, 422)


def test_get_user():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.get(f"/users/{user_id}")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == user_id


def test_get_user_not_found():
    response = client.get("/users/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}


def test_get_user_invalid_id():
    response = client.get("/users/invalid_id")
    assert response.status_code == 422
    assert (
        "Input should be a valid integer, unable to parse string as an integer"
        in response.json()["detail"][0]["msg"]
    )


def test_get_user_no_id():
    response = client.get("/users/")
    assert response.status_code == 405
    assert response.json()["detail"] == "Method Not Allowed"


def test_patch_user():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"first_name": "Jane"})
    assert response.status_code == 200
    data = response.json()
    assert data["first_name"] == "Jane"


def test_patch_user_no_fields():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={})
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == user_id


def test_patch_user_invalid_field():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"not_a_field": "value"})
    assert response.status_code in (200, 422)


def test_patch_user_invalid_type_value():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"type": "not_a_valid_type"})
    assert response.status_code == 422


def test_patch_user_type_as_int():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"type": 1})
    assert response.status_code == 422


def test_patch_user_not_found():
    response = client.patch("/users/999999", json={"first_name": "Ghost"})
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}


def test_delete_user():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.delete(f"/users/{user_id}")
    assert response.status_code == 200
    assert response.json()["ok"] is True
    # Ensure deleted
    get_resp = client.get(f"/users/{user_id}")
    assert get_resp.status_code == 404


def test_delete_user_not_found():
    response = client.delete("/users/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}


def test_delete_user_twice():
    post_resp = client.post("/users/", json=create_user_payload())
    user_id = post_resp.json()["id"]
    response = client.delete(f"/users/{user_id}")
    assert response.status_code == 200
    response2 = client.delete(f"/users/{user_id}")
    assert response2.status_code == 404


def test_post_user_type_case_sensitive():
    payload = create_user_payload(type="Family_Manager")
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_type_as_int():
    payload = create_user_payload(type=1)
    response = client.post("/users/", json=payload)
    assert response.status_code == 422
