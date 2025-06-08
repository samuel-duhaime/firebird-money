from fastapi.testclient import TestClient
from app.main import app
from app.features.categories.models import Category
from app.db import get_session
from sqlmodel import text

client = TestClient(app)

created_category_ids = []


def create_category_payload(**overrides):
    payload: Category = {
        "account_id": 1,
        "name": "Grocery",
        "type": "expense",
        "logo_url": "http://example.com/logo.png",
        "tags": [1, 2, 3],
    }
    payload.update(overrides)
    return payload


def record_category_id(response):
    if response.status_code == 200 and "id" in response.json():
        created_category_ids.append(response.json()["id"])


def test_post_category():
    response = client.post("/categories/", json=create_category_payload())
    record_category_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["account_id"] == 1
    assert data["name"] == "Grocery"
    assert data["type"] == "expense"
    assert data["logo_url"] == "http://example.com/logo.png"
    assert data["tags"] == [1, 2, 3]
    assert "id" in data


def test_post_category_minimal_fields():
    payload = {"account_id": 1, "name": "Salary", "type": "income"}
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["account_id"] == 1
    assert data["name"] == "Salary"
    assert data["type"] == "income"
    assert data["logo_url"] is None
    assert data["tags"] is None


def test_post_category_missing_account_id():
    payload = {"name": "Grocery", "type": "expense"}
    response = client.post("/categories/", json=payload)
    assert response.status_code == 422


def test_post_category_missing_name():
    payload = {"account_id": 1, "type": "expense"}
    response = client.post("/categories/", json=payload)
    assert response.status_code == 422


def test_post_category_missing_type():
    payload = {"account_id": 1, "name": "Grocery"}
    response = client.post("/categories/", json=payload)
    assert response.status_code == 422


def test_post_category_invalid_type():
    payload = create_category_payload(type="not_a_type")
    response = client.post("/categories/", json=payload)
    assert response.status_code == 422


def test_post_category_extra_fields():
    payload = create_category_payload(extra_field="should_be_ignored")
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 200
    data = response.json()
    assert "extra_field" not in data


def test_post_category_logo_url_null():
    payload = create_category_payload(logo_url=None)
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["logo_url"] is None


def test_post_category_tags_null():
    payload = create_category_payload(tags=None)
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["tags"] is None


def test_post_category_tags_empty():
    payload = create_category_payload(tags=[])
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 200
    data = response.json()
    assert data["tags"] == []


def test_get_category():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.get(f"/categories/{category_id}")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == category_id


def test_get_category_not_found():
    response = client.get("/categories/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "Category not found"}


def test_get_category_invalid_id():
    response = client.get("/categories/invalid_id")
    assert response.status_code == 422
    assert (
        "Input should be a valid integer, unable to parse string as an integer"
        in response.json()["detail"][0]["msg"]
    )


def test_patch_category():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.patch(f"/categories/{category_id}", json={"name": "Supermarket"})
    assert response.status_code == 200
    data = response.json()
    assert data["name"] == "Supermarket"


def test_patch_category_no_fields():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.patch(f"/categories/{category_id}", json={})
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == category_id


def test_patch_category_invalid_field():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.patch(f"/categories/{category_id}", json={"not_a_field": "value"})
    assert response.status_code == 200


def test_patch_category_invalid_type_value():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.patch(
        f"/categories/{category_id}", json={"type": "not_a_valid_type"}
    )
    assert response.status_code == 422


def test_patch_category_type_as_int():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.patch(f"/categories/{category_id}", json={"type": 1})
    assert response.status_code == 422


def test_patch_category_not_found():
    response = client.patch("/categories/999999", json={"name": "Ghost"})
    assert response.status_code == 404
    assert response.json() == {"detail": "Category not found"}


def test_delete_category():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.delete(f"/categories/{category_id}")
    assert response.status_code == 200
    assert response.json()["ok"] is True
    get_resp = client.get(f"/categories/{category_id}")
    assert get_resp.status_code == 404


def test_delete_category_not_found():
    response = client.delete("/categories/999999")
    assert response.status_code == 404
    assert response.json() == {"detail": "Category not found"}


def test_delete_category_twice():
    post_resp = client.post("/categories/", json=create_category_payload())
    record_category_id(post_resp)
    category_id = post_resp.json()["id"]
    response = client.delete(f"/categories/{category_id}")
    assert response.status_code == 200
    response2 = client.delete(f"/categories/{category_id}")
    assert response2.status_code == 404


def test_post_category_type_case_sensitive():
    payload = create_category_payload(type="Expense")
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 422


def test_post_category_type_as_int():
    payload = create_category_payload(type=1)
    response = client.post("/categories/", json=payload)
    record_category_id(response)
    assert response.status_code == 422


def cleanup_categories_after_tests(module):
    if created_category_ids:
        with next(get_session()) as session:
            ids_tuple = tuple(created_category_ids)
            if len(ids_tuple) == 1:
                ids_tuple = f"({ids_tuple[0]})"
            session.exec(text(f"DELETE FROM categories WHERE id IN {ids_tuple}"))
            session.commit()


teardown_module = cleanup_categories_after_tests
