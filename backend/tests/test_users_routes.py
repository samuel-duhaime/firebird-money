from fastapi.testclient import TestClient
from app.main import app
from app.features.users.models import User
from app.db import get_session
from sqlmodel import text

client = TestClient(app)

created_user_ids = []


def create_user_payload(**overrides):
    """Create a default user payload with optional overrides."""
    payload: User = {
        "account_id": 1,
        "type": "family_manager",
        "first_name": "John",
        "last_name": "Doe",
        "logo_url": "http://example.com/logo.png",
    }
    payload.update(overrides)
    return payload


def record_user_id(response):
    """Record the ID of a created user for cleanup after tests."""
    if response.status_code == 200 and "id" in response.json():
        created_user_ids.append(response.json()["id"])


def test_post_user():
    """Test creating a user with all fields."""
    response = client.post("/users/", json=create_user_payload())
    record_user_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["account_id"] == 1
    assert data["type"] == "family_manager"
    assert data["first_name"] == "John"
    assert data["last_name"] == "Doe"
    assert data["logo_url"] == "http://example.com/logo.png"
    assert "id" in data


def test_post_user_minimal_fields():
    """Test creating a user with only required fields."""
    payload = {"account_id": 1, "type": "family_member"}
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["account_id"] == 1
    assert data["type"] == "family_member"
    assert data["first_name"] is None
    assert data["last_name"] is None
    assert data["logo_url"] is None


def test_post_user_missing_account_id():
    """Test creating a user missing account_id."""
    payload = {"type": "family_manager"}
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_missing_type():
    """Test creating a user missing type."""
    payload = {"account_id": 1}
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_invalid_type():
    """Test creating a user with an invalid type."""
    payload = create_user_payload(type="not_a_type")
    response = client.post("/users/", json=payload)
    assert response.status_code == 422


def test_post_user_extra_fields():
    """Test creating a user with extra fields."""
    payload = create_user_payload(extra_field="should_be_ignored")
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200
    data = response.json()
    assert "extra_field" not in data


def test_post_user_logo_url_null():
    """Test creating a user with logo_url set to None."""
    payload = create_user_payload(logo_url=None)
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["logo_url"] is None


def test_post_user_logo_url_empty():
    """Test creating a user with logo_url set to empty string."""
    payload = create_user_payload(logo_url="")
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["logo_url"] == ""


def test_post_user_first_name_empty():
    """Test creating a user with first_name as empty string."""
    payload = create_user_payload(first_name="")
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200


def test_post_user_last_name_empty():
    """Test creating a user with last_name as empty string."""
    payload = create_user_payload(last_name="")
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200


def test_post_user_account_id_as_string():
    """Test creating a user with account_id as a string."""
    payload = create_user_payload(account_id="1")
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 200


def test_get_user():
    """Test retrieving a user by ID."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.get(f"/users/{user_id}")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == user_id


def test_get_user_not_found():
    """Test retrieving a user that does not exist."""
    response = client.get("/users/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}


def test_get_user_invalid_id():
    """Test retrieving a user with an invalid ID."""
    response = client.get("/users/invalid_id")
    assert response.status_code == 422
    assert (
        "Input should be a valid integer, unable to parse string as an integer"
        in response.json()["detail"][0]["msg"]
    )


def test_get_user_no_id():
    """Test retrieving a user with no ID (should 405)."""
    response = client.get("/users/")
    assert response.status_code == 405
    assert response.json()["detail"] == "Method Not Allowed"


def test_patch_user():
    """Test patching a user's first_name."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"first_name": "Jane"})
    assert response.status_code == 200
    data = response.json()
    assert data["first_name"] == "Jane"


def test_patch_user_no_fields():
    """Test patching a user with no fields (should be no-op)."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={})
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == user_id


def test_patch_user_invalid_field():
    """Test patching a user with an invalid field (should ignore)."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"not_a_field": "value"})
    assert response.status_code == 200


def test_patch_user_invalid_type_value():
    """Test patching a user with an invalid type value."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"type": "not_a_valid_type"})
    assert response.status_code == 422


def test_patch_user_type_as_int():
    """Test patching a user with type as an integer."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.patch(f"/users/{user_id}", json={"type": 1})
    assert response.status_code == 422


def test_patch_user_not_found():
    """Test patching a user that does not exist."""
    response = client.patch("/users/999999", json={"first_name": "Ghost"})
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}


def test_delete_user():
    """Test deleting a user."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.delete(f"/users/{user_id}")
    assert response.status_code == 200
    assert response.json()["ok"] is True
    # Ensure deleted
    get_resp = client.get(f"/users/{user_id}")
    assert get_resp.status_code == 404


def test_delete_user_not_found():
    """Test deleting a user that does not exist."""
    response = client.delete("/users/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "User not found"}


def test_delete_user_twice():
    """Test deleting a user twice."""
    post_resp = client.post("/users/", json=create_user_payload())
    record_user_id(post_resp)
    user_id = post_resp.json()["id"]
    response = client.delete(f"/users/{user_id}")
    assert response.status_code == 200
    response2 = client.delete(f"/users/{user_id}")
    assert response2.status_code == 404


def test_post_user_type_case_sensitive():
    """Test creating a user with type in wrong case."""
    payload = create_user_payload(type="Family_Manager")
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 422


def test_post_user_type_as_int():
    """Test creating a user with type as an integer."""
    payload = create_user_payload(type=1)
    response = client.post("/users/", json=payload)
    record_user_id(response)
    assert response.status_code == 422


def cleanup_users_after_tests(module):
    """Remove all users created during the tests."""
    if created_user_ids:
        with next(get_session()) as session:
            ids_tuple = tuple(created_user_ids)
            # Handle single-element tuple for SQL syntax
            if len(ids_tuple) == 1:
                ids_tuple = f"({ids_tuple[0]})"
            text(f"DELETE FROM users WHERE id IN {tuple(created_user_ids)}")
            session.commit()


# Register the cleanup function to run after all tests in this module
teardown_module = cleanup_users_after_tests
