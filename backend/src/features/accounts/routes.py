from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, EmailStr
from typing import Optional, Literal

accounts_router = APIRouter()

# In-memory store for demonstration
_fake_db = {}


class Account(BaseModel):
    id: int
    google_id: Optional[str] = None
    status: Literal["verified", "pending", "suspended"]
    email: EmailStr


class AccountCreate(BaseModel):
    email: EmailStr
    google_id: Optional[str] = None


class AccountPatch(BaseModel):
    email: Optional[EmailStr] = None
    google_id: Optional[str] = None
    status: Optional[Literal["verified", "pending", "suspended"]] = None


@accounts_router.get("/{id}", response_model=Account)
def get_account(id: int):
    account = _fake_db.get(id)
    if not account:
        raise HTTPException(status_code=404, detail="Account not found")
    return account


@accounts_router.post("/", response_model=Account)
def post_account(account: AccountCreate):
    new_id = max(_fake_db.keys(), default=0) + 1
    acc = Account(id=new_id, status="pending", **account.model_dump())
    _fake_db[new_id] = acc
    return acc


@accounts_router.patch("/{id}", response_model=Account)
def patch_account(id: int, fields: AccountPatch):
    account = _fake_db.get(id)
    if not account:
        raise HTTPException(status_code=404, detail="Account not found")
    updated = account.model_copy(update=fields.model_dump(exclude_unset=True))
    _fake_db[id] = updated
    return updated


@accounts_router.delete("/{id}")
def delete_account(id: int):
    if id not in _fake_db:
        raise HTTPException(status_code=404, detail="Account not found")
    del _fake_db[id]
    return {"ok": True}
